use actix_session::Session;
use actix_web::{get, http::header::ContentType, web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use tracing_subscriber::fmt::format;
use uuid::Uuid;

fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

#[get("/admin/dashboard")]
#[tracing::instrument(name = "Redirecting to admin page", skip(session, pool))]
pub async fn admin_dashboard(
    session: Session,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get::<Uuid>("user_id").map_err(e500)? {
        get_username(user_id, &pool).await.map_err(e500)?
    } else {
        todo!()
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
    <html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Admin dashboard</title>
    </head>
    <body>
        <p>Welcome {username}!</p>
    </body>
    </html>"#
        )))
}

#[tracing::instrument(name = "Fetching user to get the name", skip(pool))]
async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(r#"SELECT username FROM users where user_id = $1"#, user_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform a query to retrive username")?;
    Ok(row.username)
}
