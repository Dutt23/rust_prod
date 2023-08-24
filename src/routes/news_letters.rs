use crate::{authentication::AuthError, domains::SubscriberEmail, email_client::EmailClient};
use actix_web::{
    http::header::{self, HeaderMap},
    post, web, HttpRequest, HttpResponse, ResponseError,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::{engine::general_purpose, Engine as _};
use reqwest::StatusCode;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
//  format! allocates memory on the heap to store its output
use super::error_chain_fmt;
use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;

#[derive(serde::Deserialize)]
pub struct NewsLetter {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    text: String,
    html: String,
}

struct ConfirmedSubscriber {
    email: String,
}

pub struct Credentials {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnExceptedError(#[from] anyhow::Error),
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            PublishError::UnExceptedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PublishError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnExceptedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => HttpResponse::build(StatusCode::UNAUTHORIZED)
                .append_header((header::WWW_AUTHENTICATE, r#"Basic realm="publish""#))
                .finish(),
        }
    }
}

#[tracing::instrument(
    name = "Publish a news letter",
    skip(news_letter, email_client, pool, request)
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
#[post("/newsletter")]
pub async fn publish_newsletter(
    news_letter: web::Json<NewsLetter>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    let subscribers = get_confirmed_subscribers(&pool).await?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentialsError(_) => PublishError::AuthError(e.into()),
            AuthError::UnExceptedError(_) => PublishError::AuthError(e.into()),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    for subscriber in subscribers {
        match subscriber {
            Ok(email) => email_client
                .send_email(
                    &email,
                    &news_letter.title,
                    &news_letter.content.html,
                    &news_letter.content.text,
                )
                .await
                .with_context(|| format!("Unable to send email to {}", email.as_ref()))?,
            Err(error) => {
                tracing::warn!(
                    error.error_chain = ?error,
                    "Skipping confirmed subscriber. \
                    Their stored contact details are invalid "
                )
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get a list of confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<SubscriberEmail, String>>, anyhow::Error> {
    Ok(sqlx::query_as!(
        ConfirmedSubscriber,
        r#"SELECT email from subscriptions where status = 'confirmed'"#
    )
    .fetch_all(pool)
    .await?
    .iter()
    .map(|row| SubscriberEmail::parse(row.email.clone()))
    .collect())
}

#[tracing::instrument(name = "Validating credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let user = get_stored_credentials(&credentials.username, pool)
        .await
        .map_err(AuthError::UnExceptedError)?
        .ok_or_else(|| AuthError::InvalidCredentialsError(anyhow::anyhow!("Incorrect username")))?;

    spawn_blocking_with_tracing(move || verify_password(user.1, credentials.password))
        .await
        .context("Passwords do not match")
        .map_err(AuthError::InvalidCredentialsError)??;

    Ok(user.0)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password, password_candidate)
)]
pub fn verify_password(
    expected_password: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let password_hash = PasswordHash::new(&expected_password.expose_secret())
        .context("Unable to parse password string")?;

    tracing::info_span!("Verify password hash")
        .in_scope(|| {
            Argon2::default().verify_password(
                password_candidate.expose_secret().as_bytes(),
                &password_hash,
            )
        })
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentialsError)
}

#[tracing::instrument(name = "Fetch stored user", skip(pool))]
pub async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let user = sqlx::query!(
        r#"SELECT user_id, password_hash from users where username = $1"#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to retrieve user")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(user)
}

pub fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header = headers
        .get("Authorization")
        .context("The 'Authorization header was missing'")?
        .to_str()
        .context("The Authorization header was not a valid string")?;

    let encoded_segment = header
        .strip_prefix("Basic ")
        .context("Authorization header did not match")?;

    let decoded_bytes = &general_purpose::STANDARD
        .decode(encoded_segment)
        .context("Authorization data not encoded properly")?;

    let decoded_credentials =
        String::from_utf8(decoded_bytes.to_owned()).context("Unable to decode properly")?;

    let mut credentials = decoded_credentials.splitn(2, ":");

    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!(" A username must be provided in 'Basic' auth"))?
        .to_string();

    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!(" A password must be provided "))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}
