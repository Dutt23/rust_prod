use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::{confirm, health_check, publish_newsletter, subscriptions};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    server: Server,
    port: u16,
}

impl Application {
    pub async fn build(settings: &Settings) -> Result<Self, std::io::Error> {
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
        )?;

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

pub struct ApplicationBaseUrl(pub String);

// https://ryhl.io/blog/async-what-is-blocking/
fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    // TODO: https://stackoverflow.com/questions/71497831/is-there-a-way-to-split-server-routes-declaration-in-actix-web
    // Wraps it in an Arc
    let conn = web::Data::new(db_pool);
    let e_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url.clone()));
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(conn.clone())
            .app_data(e_client.clone())
            .app_data(base_url.clone())
            .service(subscriptions)
            .service(health_check)
            .service(confirm)
            .service(publish_newsletter)
    })
    .listen(listener)?
    .run();

    Ok(server)
}
