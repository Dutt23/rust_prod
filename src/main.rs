use std::fmt::{Debug, Display};

use news_letter::{
    configuration::get_configuration,
    issue_delivery_worker::run_worker_until_stopped,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};
use tokio::task::JoinError;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let settings = get_configuration().expect("Unable to read configuration files");
    let subscriber = get_subscriber("news_letter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let app = Application::build(&settings)
        .await
        .expect("Unable to build application");
    let worker = run_worker_until_stopped(settings);

    let application_task = tokio::spawn(app.run_until_stopped());
    let worker_task = tokio::spawn(worker);
    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("BACKGROUN_WORKER", o),
    };
    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(      error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name)
        }
        Err(e) => {
            tracing::error!(      error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name)
        }
    }
}
