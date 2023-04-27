use news_letter::{
    configuration::get_configuration,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings = get_configuration().expect("Unable to read configuration files");
    let subscriber = get_subscriber("news_letter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let app = Application::build(&settings)
        .await
        .expect("Unable to build application");

    app.run_until_stopped().await?;
    Ok(())
}
