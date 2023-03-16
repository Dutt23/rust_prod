use actix_web::{post, web, Responder};

#[derive(serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

#[post("/subscriptions")]
async fn subscriptions(form: web::Form<FormData>) -> impl Responder {
    format!("Welcome , {}", form.name)
}
