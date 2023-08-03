use actix_web::{get, web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirming a subscriber", skip(_parameters))]
#[get("/subscriptions/confirm")]
pub async fn confirm(_parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    let subscription_id =
        match get_subscription_id_from_token(&pool, &_parameters.subscription_token).await {
            Ok(subscription_id) => subscription_id,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    match subscription_id {
        None => return HttpResponse::InternalServerError().finish(),
        Some(subscription_id) => {
            if confirm_subscription_token(&pool, &subscription_id)
                .await
                .is_err()
            {
                return HttpResponse::InternalServerError().finish();
            }
        }
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Confirming a subscription in the database", skip(pool))]
pub async fn confirm_subscription_token(
    pool: &PgPool,
    subscription_id: &Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscription_id
    )
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {:?}", err);
        err
    })?;
    Ok(())
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
