use crate::{
    authentication::UserId, domains::SubscriberEmail, email_client::EmailClient,
    routes::error_chain_fmt,
};
use actix_web::{
    http::header::{self},
    post, web, HttpRequest, HttpResponse, ResponseError,
};

use anyhow::Context;
use reqwest::StatusCode;
use sqlx::PgPool;

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

struct ConfirmedSubscriber {
    email: String,
}

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(
    name = "Publish a news letter",
    skip_all,
    fields(user_id=%&*user_id)
)]
#[post("/newsletter")]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, PublishError> {
    dbg!("Inside here");
    let subscribers = get_confirmed_subscribers(&pool).await?;
    let FormData {
        title,
        text_content,
        html_content,
    } = form.0;
    tracing::Span::current().record("user_id", &tracing::field::display(*user_id));
    for subscriber in subscribers {
        match subscriber {
            Ok(email) => email_client
                .send_email(&email, &title, &html_content, &text_content)
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
