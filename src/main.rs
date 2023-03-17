use env_logger::Env;
use news_letter::{configuration::get_configuration, startup::*};
use sqlx::PgPool;
use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings = get_configuration().expect("Unable to read configuration files");

    let connection_pool = PgPool::connect(&settings.database.get_connection_string())
        .await
        .expect("Failed to connect to Postgres.");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", settings.application_port))?;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    run(listener, connection_pool)?.await
}
