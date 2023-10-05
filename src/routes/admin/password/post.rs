use actix_web::{post, web, Error, HttpResponse};
use secrecy::Secret;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[post("/admin/password")]
pub async fn change_password(form: web::Form<FormData>) -> Result<HttpResponse, Error> {
    todo!()
}
