use crate::{
    domains::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};
use actix_web::{post, web, HttpResponse, Responder};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::PgPool;
use uuid::Uuid;

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

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

// Spans, like logs, have an associated level // `info_span` creates a span at the info-level
// See the following section on `Instrumenting Futures`
#[tracing::instrument(name = "Adding a new subscriber.", skip(form, pool,  email_client, base_url), fields(subscriber_email = %form.email, subscriber_name= %form.name))]
#[post("/subscriptions")]
async fn subscriptions(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> impl Responder {
    let new_subscriber = match form.0.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let subscription_token = generate_subscription_token();

    let subscription_id = match insert(&pool, &new_subscriber).await {
        Ok(subscription_id) => subscription_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    if store_token(&pool, &subscription_id, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    tracing::info!("New subscriber details have been saved");
    if send_confirmation_email_to_customer(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .is_err()
    {
        HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Sending confirmation email to customer",
    skip(email_client, new_subscriber)
)]
async fn send_confirmation_email_to_customer(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &String,
    subscription_token: &String,
) -> Result<(), reqwest::Error> {
    let confirmation_link =
        &format!("{base_url}/subscriptions/confirm?subscription_token={subscription_token}");
    let plain_body = &format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = &format!(
        "Welcome to our newsletter!<br />\
		Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, "Welcome!", &plain_body, html_body)
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, pool)
)]
pub async fn insert(pool: &PgPool, new_subscriber: &NewSubscriber) -> Result<Uuid, sqlx::Error> {
    let subscription_id = Uuid::new_v4();
    sqlx::query!(
        r#"
   INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')
   "#,
   subscription_id,
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

    Ok((subscription_id))
}

#[tracing::instrument(
    name = "Storing subscription token in the database",
    skip(subscription_id, pool)
)]
pub async fn store_token(
    pool: &PgPool,
    subscription_id: &Uuid,
    subscription_token: &String,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT into subscription_tokens (subscription_token, subscription_id) VALUES ($1, $2)"#,
        subscription_token,
        subscription_id
    )
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("Error happened while executing query :{:?}", err);
        err
    })?;
    Ok(())
}
