use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::error_chain_fmt,
    state_session::TypedSession,
};

use actix_web::{
    cookie::{time::Duration, Cookie},
    error::InternalError,
    post, web, HttpResponse,
};
use actix_web_flash_messages::FlashMessage;
use hmac::{Hmac, Mac};
use reqwest::header::LOCATION;
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::PgPool;
#[derive(serde::Deserialize)]
pub struct FormData {
    pub username: String,
    pub password: String,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

#[tracing::instrument(name = "Logging in a new user.", skip(form_data, pool, session))]
#[post("/login")]
pub async fn login(
    form_data: web::Form<FormData>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form_data.0.username,
        password: Secret::new(form_data.0.password),
    };

    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| {
            let err = match e {
                AuthError::InvalidCredentialsError(_) => LoginError::AuthError(e.into()),
                AuthError::UnExceptedError(_) => LoginError::UnexpectedError(e.into()),
            };

            let location = "/login";
            FlashMessage::error(err.to_string()).send();
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, location))
                .finish();
            InternalError::from_response(err, response)
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    // session.renew();
    // Serializes here, could result in an error
    session
        .insert_user_id(user_id)
        .map_err(|e| login_redirect(LoginError::UnexpectedError(e.into())))?;
    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/admin/dashboard"))
        .finish())
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(e.to_string()).send();
    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish();
    InternalError::from_response(e, response)
}

pub fn get_encoded_string(url: &str, message: String, secret: &Secret<String>) -> String {
    let query_string = format!("error={}", urlencoding::Encoded::new(message.to_string()));
    let hmac_tag = {
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.finalize().into_bytes()
    };

    format!("{url}?{query_string}&tag={hmac_tag:x}")
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
