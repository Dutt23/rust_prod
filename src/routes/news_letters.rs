use actix_web::{post, web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct NewsLetter {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    text: String,
    html: String,
}

#[tracing::instrument(name = "Publish a news letter", skip(_body))]
#[post("/newsletter")]
pub async fn publish_newsletter(_body: web::Json<NewsLetter>) -> HttpResponse {
    return HttpResponse::Ok().finish();
}
