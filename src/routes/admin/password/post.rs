use crate::{routes::admin::dashboard::e500, state_session::TypedSession};
use actix_web::{post, web, Error, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use reqwest::header::LOCATION;
use secrecy::{ExposeSecret, Secret};

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

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/admin/password"))
            .finish());
    }
    todo!()
}
