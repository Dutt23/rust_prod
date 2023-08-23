use actix_web::{post, HttpResponse};
use reqwest::header::LOCATION;

#[post("/login")]
pub async fn login() -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish()
}
