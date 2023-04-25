use actix_web::{post, web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::{NewSubscriber, SubscriberEmail, SubscriberName};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(NewSubscriber { email, name })
    }
}

// Spans, like logs, have an associated level // `info_span` creates a span at the info-level
// See the following section on `Instrumenting Futures`
#[tracing::instrument(name = "Adding a new subscriber.", skip(form, pool), fields(subscriber_email = %form.email, subscriber_name= %form.name))]
#[post("/subscriptions")]
async fn subscriptions(form: web::Form<FormData>, pool: web::Data<PgPool>) -> impl Responder {
    let new_subscriber = match form.0.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match insert(&pool, &new_subscriber).await {
        Ok(_) => {
            tracing::info!("New subscriber details have been saved");
            HttpResponse::Ok().finish()
        }
        Err(err) => {
            tracing::error!("Error occured while saving : {}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, pool)
)]
pub async fn insert(pool: &PgPool, new_subscriber: &NewSubscriber) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
   INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)
   "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    // We use `get_ref` to get an immutable reference to the `PgConnection`
    // wrapped by `web::Data`.
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("Error happened while executing query :{:?}", err);
        err
    })?;

    Ok(())
}
