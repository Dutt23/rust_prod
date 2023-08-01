use news_letter::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub app_port: u16,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber("test".into(), "debug".into(), std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber("test".into(), "debug".into(), std::io::sink);
        init_subscriber(subscriber);
    }
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("APP_ENVIRONMENT", "test");

    let email_server = MockServer::start().await;
    let configuration = {
        let mut config = get_configuration().expect("Unable to read configuration files");
        config.database.database_name = Uuid::new_v4().to_string();
        // Use random os port
        config.application.port = 0;
        config.email_client.base_url = email_server.uri();
        config
    };

    let app = Application::build(&configuration)
        .await
        .expect("Failed to build application");

    configure_database(&configuration.database).await;
    let address = format!("http://127.0.0.1:{}", app.port());
    let application_port = app.port();
    let _ = tokio::spawn(app.run_until_stopped());

    TestApp {
        address,
        db_pool: get_connection_pool(&configuration),
        email_server,
        app_port: application_port,
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // The first time `initialize` is invoked the code in `TRACING` is executed. // All other invocations will instead skip execution.

    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres.");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    return connection_pool;
}
