use crate::{authentication::UserId, routes::admin::dashboard::e500, state_session::TypedSession};
use actix_web::{post, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use reqwest::header::LOCATION;

fn see_other(route: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, route))
        .finish()
}

#[post("/logout")]
#[tracing::instrument(name = "Logging out user")]
pub async fn log_out(user_id: web::ReqData<UserId>) -> Result<HttpResponse, actix_web::Error> {
    Ok(see_other("/login"))
}
