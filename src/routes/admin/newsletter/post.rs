use crate::idempotency::{get_saved_response, try_processing, NextAction};
use crate::routes::e500;
use crate::{
    authentication::UserId, domains::SubscriberEmail, email_client::EmailClient,
    idempotency::save_response, idempotency::IdempotencyKey, routes::error_chain_fmt, utils::e400,
};
use actix_web::{
    http::header::{self},
    post, web, HttpResponse, ResponseError,
};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use reqwest::header::LOCATION;
use reqwest::StatusCode;
use sqlx::PgPool;

fn see_other(route: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, route))
        .finish()
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

struct ConfirmedSubscriber {
    email: String,
}

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

#[tracing::instrument(
    name = "Publish a news letter",
    skip_all,
    fields(user_id=%&*user_id)
)]
#[post("/newsletters")]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    dbg!("Inside here");
    let user_id = user_id.into_inner();
    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e400)?;
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;

    let idempotency_key: IdempotencyKey = idempotency_key
        .try_into()
        .with_context(|| format!("Unable to get idempotency key"))
        .map_err(e400)?;
    let transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    tracing::Span::current().record("user_id", &tracing::field::display(*user_id));
    for subscriber in subscribers {
        match subscriber {
            Ok(email) => email_client
                .send_email(&email, &title, &html_content, &text_content)
                .await
                .with_context(|| format!("Unable to send email to {}", email.as_ref()))
                .map_err(e400)?,
            Err(error) => {
                tracing::warn!(
                    error.error_chain = ?error,
                    "Skipping confirmed subscriber. \
                    Their stored contact details are invalid "
                )
            }
        }
    }

   success_message().send();
    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published!")
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
