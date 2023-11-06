use crate::state_session::TypedSession;
use actix_web::{post, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use reqwest::header::LOCATION;

fn see_other(route: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, route))
        .finish()
}

#[post("/logout")]
#[tracing::instrument(name = "Logging out user", skip(session))]
pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    session.log_out();
    FlashMessage::info("You have successfully logged out.").send();
    Ok(see_other("/login"))
}
