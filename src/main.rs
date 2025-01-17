mod api;
mod database;
mod models;
mod repo;
mod schema;

use tokio::net::TcpListener;
use tracing::info;

use api::build_app;
use database::{create_db_pool, DatabaseBookRepo};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let repo = DatabaseBookRepo::new(create_db_pool().await);

    let app = build_app(repo);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    info!("Listening on {}", local_addr);

    axum::serve(listener, app).await.unwrap();
}
