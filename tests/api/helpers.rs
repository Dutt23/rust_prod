use news_letter::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};
use once_cell::sync::Lazy;
use reqwest::Url;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub app_port: u16,
}

pub struct ConfirmationLink {
    pub html: Url,
    pub plain_text: Url,
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

    pub fn get_confirmation_links(&self, email_req: &wiremock::Request) -> ConfirmationLink {
        let body: serde_json::Value = serde_json::from_slice(&email_req.body).unwrap();

        ConfirmationLink {
            html: self.get_links(&body["HtmlBody"].as_str().unwrap()),
            plain_text: self.get_links(&body["TextBody"].as_str().unwrap()),
        }
    }

    fn get_links(&self, s: &str) -> Url {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        let raw_link = links[0].as_str().to_owned();
        let mut confirmation_link = Url::parse(&raw_link).unwrap();
        assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
        confirmation_link.set_port(Some(self.app_port)).unwrap();
        confirmation_link
    }

    pub async fn post_news_letters(&self, json: &serde_json::Value) -> reqwest::Response {
        let (username, password) = self.get_test_user().await;
        reqwest::Client::new()
            .post(format!("{}/newsletter", &self.address))
            .json(json)
            .basic_auth(username, Some(password))
            .send()
            .await
            .expect("Unable to send request")
    }

    async fn get_test_user(&self) -> (String, String) {
        let row = sqlx::query!("SELECT username, password FROM users LIMIT 1",)
            .fetch_one(&self.db_pool)
            .await
            .expect("Unable to fetch test users");
        (row.username, row.password)
    }
}

async fn add_test_user(pool: &PgPool) {
    sqlx::query!(
        "INSERT into users (user_id, username, password) VALUES ($1, $2, $3)",
        Uuid::new_v4(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string()
    )
    .execute(pool)
    .await
    .expect("Failed to create users");
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
    let db_pool = get_connection_pool(&configuration);
    add_test_user(&db_pool).await;

    TestApp {
        address,
        db_pool,
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
