use std::net::TcpListener;
use news_letter::startup::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:3000")?;
    run(listener)?.await
}
