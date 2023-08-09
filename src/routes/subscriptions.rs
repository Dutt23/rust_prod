use crate::{
    domains::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};
use actix_web::{post, web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use reqwest::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use anyhow::Context;

#[derive(thiserror::Error, Debug)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnExceptedError(#[from] anyhow::Error), // #[error("Failed to store the confirmation token for a new subscriber.")]
                                            // StoreTokenError(#[from] StoreTokenError),
                                            // #[error("Failed to send a confirmation email.")]
                                            // SendEmailError(#[from] reqwest::Error),
                                            // #[error("Failed to accquire a postgres connection from the pool")]
                                            // PoolError(#[source] sqlx::Error),
                                            // #[error("Failed to insert new subscriber in the database.")]
                                            // InsertSubscriberErrors(#[source] sqlx::Error),
                                            // #[error("Failed to commit SQL transaction to store a new subscriber.")]
                                            // TransactionCommitError(#[source] sqlx::Error),
}

#[derive(Debug)]
pub struct StoreTokenError(sqlx::Error);

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database failure was encountered while trying to store a subscription token."
        )
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnExceptedError(_) => StatusCode::INTERNAL_SERVER_ERROR
            // SubscribeError::InsertSubscriberErrors(_)
            // | SubscribeError::PoolError(_)
            // | SubscribeError::TransactionCommitError(_)
            // | SubscribeError::StoreTokenError(_)
            // | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

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
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;
    let subscription_token = generate_subscription_token();
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let subscription_id = insert(&new_subscriber, &mut transaction)
        .await
        .context("Unable to inert subscriber into database")?;
    // .map_err(SubscribeError::InsertSubscriberErrors)?;

    store_token(&subscription_id, &subscription_token, &mut transaction)
        .await
        .context("Unable to store token for subscriber")?;
    // .map_err(SubscribeError::StoreTokenError)?;

    transaction
        .commit()
        .await
        .context("Unable to save transaction")?;

    tracing::info!("New subscriber details have been saved");
    send_confirmation_email_to_customer(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send confirmation Email to customer")?;

    Ok(HttpResponse::Ok().finish())
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
    skip(new_subscriber, transaction)
)]
pub async fn insert(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
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
    .execute(transaction)
    .await?;

    Ok(subscription_id)
}

#[tracing::instrument(
    name = "Storing subscription token in the database",
    skip(subscription_id, transaction)
)]
pub async fn store_token(
    subscription_id: &Uuid,
    subscription_token: &String,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT into subscription_tokens (subscription_token, subscription_id) VALUES ($1, $2)"#,
        subscription_token,
        subscription_id
    )
    .execute(transaction)
    .await?;
    Ok(())
}
