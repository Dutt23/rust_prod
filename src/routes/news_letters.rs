use actix_web::{post, HttpResponse};

#[tracing::instrument(name = "Publish a news letter")]
#[post("/newsletter")]
pub async fn publish_newsletter() -> HttpResponse {
    return HttpResponse::Ok().finish();
}
