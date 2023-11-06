use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    routes::admin::dashboard::{e500, get_username},
    state_session::TypedSession,
};
use actix_web::{post, web, Error, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use reqwest::header::LOCATION;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

fn see_other(route: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, route))
        .finish()
}

#[post("/admin/password")]
pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let user_id = session.get_user_id().map_err(e500)?;
    if user_id.is_none() {
        return Ok(see_other("/login"));
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

    let username = get_username(user_id.unwrap(), &pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentialsError(_) => {
                FlashMessage::error("The current password must be valid").send();
                return Ok(see_other("/admin/password"));
            }
            AuthError::UnExceptedError(_) => Err(e500(e).into()),
        };
    }
    crate::authentication::change_password(user_id.unwrap(), form.0.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
