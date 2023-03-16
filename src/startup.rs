use std::net::TcpListener;
use actix_web::dev::Server;
use actix_web::{ App, HttpServer};

use crate::routes::{subscriptions, health_check};

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().service(subscriptions).service(health_check))
        .listen(listener)?
        .run();

    Ok(server)
}