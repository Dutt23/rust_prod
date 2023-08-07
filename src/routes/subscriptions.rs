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

#[derive(Debug)]
pub enum SubscribeError {
    ValidationError(String),
    DatabaseError(sqlx::Error),
    StoreTokenError(StoreTokenError),
    SendEmailError(reqwest::Error),
}

#[derive(Debug)]
pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscribeError::ValidationError(e) => write!(f, "Validation failure {}", e),
            SubscribeError::DatabaseError(_) => write!(f, "????"),
            SubscribeError::StoreTokenError(_) => write!(
                f,
                "Failed to store the confirmation token for a new subscriber."
            ),
            SubscribeError::SendEmailError(_) => write!(f, "Unable to send confirmation email"),
        }
    }
}

impl From<StoreTokenError> for SubscribeError {
    fn from(e: StoreTokenError) -> Self {
        Self::StoreTokenError(e)
    }
}

impl From<String> for SubscribeError {
    fn from(e: String) -> Self {
        Self::ValidationError(e)
    }
}

impl From<reqwest::Error> for SubscribeError {
    fn from(e: reqwest::Error) -> Self {
        Self::SendEmailError(e)
    }
}

impl From<sqlx::Error> for SubscribeError {
    fn from(e: sqlx::Error) -> Self {
        Self::DatabaseError(e)
    }
}

impl std::error::Error for SubscribeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SubscribeError::ValidationError(_) => None,
            SubscribeError::DatabaseError(e) => Some(e),
            SubscribeError::SendEmailError(e) => Some(e),
            SubscribeError::StoreTokenError(e) => Some(&e.0),
        }
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::DatabaseError(_)
            | SubscribeError::StoreTokenError(_)
            | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
    let new_subscriber = form.0.try_into()?;
    let subscription_token = generate_subscription_token();
    let mut transaction = pool.begin().await?;

    let subscription_id = insert(&new_subscriber, &mut transaction).await?;
    store_token(&subscription_id, &subscription_token, &mut transaction).await?;

    transaction.commit().await?;

    tracing::info!("New subscriber details have been saved");
    send_confirmation_email_to_customer(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await?;

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
    .await
    .map_err(|err| {
        tracing::error!("Error happened while executing query :{:?}", err);
        err
    })?;

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
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"INSERT into subscription_tokens (subscription_token, subscription_id) VALUES ($1, $2)"#,
        subscription_token,
        subscription_id
    )
    .execute(transaction)
    .await
    .map_err(|err| {
        tracing::error!("Error happened while executing query :{:?}", err);
        StoreTokenError(err)
    })?;
    Ok(())
}
