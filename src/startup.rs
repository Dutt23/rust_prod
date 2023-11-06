use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{
    admin_dashboard, change_password, change_password_form, confirm, health_check, home, log_out,
    login, login_form, publish_newsletter, subscriptions,
};
use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::dev::Server;
use actix_web::{cookie::Key, web, App, HttpServer};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use secrecy::ExposeSecret;
use secrecy::Secret;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    server: Server,
    port: u16,
}

impl Application {
    pub async fn build(settings: &Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(settings);
        let sender_email = settings
            .email_client
            .sender()
            .expect("Unable to get sender email");

        let timeout = settings.email_client.timeout();
        let email_client = EmailClient::new(
            settings.email_client.base_url.to_owned(),
            sender_email,
            settings.email_client.authorization_token.to_owned(),
            timeout,
        );
        let listener = TcpListener::bind(format!("127.0.0.1:{}", settings.application.port))?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            settings.application.base_url.to_owned(),
            settings.application.hmac_secret.to_owned(),
            settings.redis_uri.to_owned(),
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(settings: &Settings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(settings.database.with_db())
}
#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);

pub struct ApplicationBaseUrl(pub String);

// https://ryhl.io/blog/async-what-is-blocking/
async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    // TODO: https://stackoverflow.com/questions/71497831/is-there-a-way-to-split-server-routes-declaration-in-actix-web
    // Wraps it in an Arc
    let conn = web::Data::new(db_pool);
    let e_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url.clone()));
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let cookie_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(cookie_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .app_data(conn.clone())
            .app_data(e_client.clone())
            .app_data(base_url.clone())
            .service(home)
            .service(login_form)
            .service(login)
            .service(subscriptions)
            .service(health_check)
            .service(confirm)
            .service(publish_newsletter)
            .service(admin_dashboard)
            .service(change_password)
            .service(change_password_form)
            .service(log_out)
    })
    .listen(listener)?
    .run();

    Ok(server)
}
