use actix_web::{post, web, HttpResponse, ResponseError};
use reqwest::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};

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

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error(transparent)]
    UnExceptedError(#[from] anyhow::Error),
}

impl ResponseError for PublishError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            PublishError::UnExceptedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Publish a news letter", skip(_body))]
#[post("/newsletter")]
pub async fn publish_newsletter(
    _body: web::Json<NewsLetter>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, PublishError> {
    let _subscribers = get_confirmed_subscribers(&pool).await?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get a list of confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<ConfirmedSubscriber>, anyhow::Error> {
    let rows = sqlx::query_as!(
        ConfirmedSubscriber,
        r#"SELECT email from subscriptions where status = 'confirmed'"#
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
