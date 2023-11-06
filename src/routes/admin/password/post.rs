use actix_web::{post, web, Error, HttpResponse};
use reqwest::header::LOCATION;
use secrecy::Secret;

use crate::{routes::admin::dashboard::e500, state_session::TypedSession};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[post("/admin/password")]
pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
) -> Result<HttpResponse, Error> {
    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    }
    todo!()
}
