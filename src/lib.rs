mod api;
mod database;
mod models;
mod repo;
mod schema;

use axum::{serve::Serve, Router};
use tokio::net::TcpListener;
use tracing::info;

use api::build_api;
use database::{create_db_pool, DatabaseBookRepo};

pub async fn start_server(db_url: String) -> Serve<TcpListener, Router, Router> {
    let repo = DatabaseBookRepo::new(create_db_pool(db_url).await);

    let router = build_api(repo);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    info!("Listening on {}", local_addr);

    axum::serve(listener, router)
}
