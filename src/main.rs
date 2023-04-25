use news_letter::{
    configuration::get_configuration,
    startup::*,
    telemetry::{get_subscriber, init_subscriber},
};
use sqlx::PgPool;
use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings = get_configuration().expect("Unable to read configuration files");

    let connection_pool = PgPool::connect_lazy(&settings.database.get_connection_string())
        .expect("Failed to connect to Postgres.");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", settings.application.port))?;
    let subscriber = get_subscriber("news_letter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    run(listener, connection_pool)?.await
}
