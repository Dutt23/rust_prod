use crate::telemetry::spawn_blocking_with_tracing;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use anyhow::Context;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid Credentials")]
    InvalidCredentialsError(#[source] anyhow::Error),
    #[error(transparent)]
    UnExceptedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
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
