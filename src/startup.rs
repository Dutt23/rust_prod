use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;

use crate::routes::{health_check, subscriptions};

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    // Wraps it in an Arc
    let conn = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(conn.clone())
            .service(subscriptions)
            .service(health_check)
    })
    .listen(listener)?
    .run();

    Ok(server)
}
