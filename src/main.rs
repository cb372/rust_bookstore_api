use rust_bookstore_api::start_server;
use std::env;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_url = env::var("DATABASE_URL").unwrap_or("postgres://localhost/bookstore".to_string());

    let server = start_server(db_url).await;

    server.await.unwrap();
}
