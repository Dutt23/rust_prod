use news_letter::{configuration::get_configuration, startup::*};
use std::net::TcpListener;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings = get_configuration().expect("Unable to read configuration files");
    let listener = TcpListener::bind(format!("127.0.0.1:{}", settings.application_port))?;
    run(listener)?.await
}
