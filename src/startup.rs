use crate::email_client::EmailClient;
use crate::routes::{health_check, subscriptions};

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    // Wraps it in an Arc
    let conn = web::Data::new(db_pool);
    let e_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(conn.clone())
            .app_data(e_client.clone())
            .service(subscriptions)
            .service(health_check)
    })
    .listen(listener)?
    .run();

    Ok(server)
}
