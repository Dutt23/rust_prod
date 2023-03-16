use actix_web::{post, web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
struct FormData {
    email: String,
    name: String,
}

#[post("/subscriptions")]
async fn subscriptions(form: web::Form<FormData>, pool: web::Data<PgPool>) -> impl Responder {
    match sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at) VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    // We use `get_ref` to get an immutable reference to the `PgConnection`
    // wrapped by `web::Data`.
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            print!("Error occured while saving : {}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}
