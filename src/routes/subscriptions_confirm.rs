use actix_web::{get, HttpResponse};

#[tracing::instrument(name = "Confirming a subscriber")]
#[get("/subscription/confirm")]
pub async fn confirm() -> HttpResponse {
    HttpResponse::Ok().finish()
}
