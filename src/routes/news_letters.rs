use actix_web::{post, web, HttpResponse};

#[derive(serde::Deserialize)]
struct NewsLetter {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
struct Content {
    text: String,
    html: String,
}

#[tracing::instrument(name = "Publish a news letter")]
#[post("/newsletter")]
pub async fn publish_newsletter(_body: web::Json<NewsLetter>) -> HttpResponse {
    return HttpResponse::Ok().finish();
}
