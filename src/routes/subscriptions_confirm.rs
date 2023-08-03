use actix_web::{get, web, HttpResponse};
use sqlx::{pool, PgPool};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirming a subscriber", skip(_parameters))]
#[get("/subscriptions/confirm")]
pub async fn confirm(_parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Retrieving a subscription token from database",
    skip(pool, subscription_token)
)]
pub async fn get_subscription_id_from_token(
    pool: &PgPool,
    subscription_token: &String,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscription_id from subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);
        err
    })?;
    return Ok(result.map(|r| r.subscription_id));
}
