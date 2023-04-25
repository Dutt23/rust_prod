use news_letter::{configuration::get_configuration, startup::*};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber(name: String, env_filter: String) -> impl Subscriber + Send + Sync {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let formatting_layer = BunyanFormattingLayer::new(
        name,
        // Output the formatted spans to stdout.
        std::io::stdout,
    );

    // The `with` method is provided by `SubscriberExt`, an extension // trait for `Subscriber` exposed by `tracing_subscriber`
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    LogTracer::init().expect("Failed to set logger");
    let settings = get_configuration().expect("Unable to read configuration files");

    let connection_pool = PgPool::connect(&settings.database.get_connection_string())
        .await
        .expect("Failed to connect to Postgres.");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", settings.application_port))?;
    let subscriber = get_subscriber("news_letter".into(), "info".into());
    init_subscriber(subscriber);
    run(listener, connection_pool)?.await
}
