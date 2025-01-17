mod database;
mod models;
mod repo;
mod schema;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use database::{create_db_pool, DatabaseBookRepo};
use models::{Book, NewBook};
use repo::BookRepo;
use std::error::Error;
use tokio::net::TcpListener;
use tracing::info;

#[derive(Clone)]
struct AppState<R> {
    repo: R,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let repo = DatabaseBookRepo::new(create_db_pool().await);

    let app = Router::new()
        .route("/books", get(list_books).post(insert_book))
        .route(
            "/books/{id}",
            get(get_book).put(update_book).delete(delete_book),
        )
        .with_state(AppState { repo });

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    info!("Listening on {}", local_addr);

    axum::serve(listener, app).await.unwrap();
}

// TODO I want the repo to be abstract `impl BookRepo<E>`, but I couldn't get it to compile

async fn list_books<E, R>(
    State(state): State<AppState<R>>,
) -> Result<Json<Vec<Book>>, (StatusCode, String)>
where
    E: Error,
    R: BookRepo<E>,
{
    // TODO pagination
    let results = state.repo.list_books().await.map_err(internal_error)?;

    info!("Retrieved {} books from the DB", results.len());

    Ok(Json(results))
}

async fn get_book<E, R>(
    State(state): State<AppState<R>>,
    Path(id): Path<String>,
) -> Result<Json<Book>, (StatusCode, String)>
where
    E: Error,
    R: BookRepo<E>,
{
    let id = parse_book_id(id)?;

    let book = state.repo.get_book(id).await.map_err(internal_error)?;

    match book {
        Some(book) => {
            info!("Retrieved book from DB: {:?}", book);
            Ok(Json(book))
        }
        None => {
            info!("No book found in DB with ID: {}", id);
            Err((
                StatusCode::NOT_FOUND,
                format!("No book found with ID: {}", id),
            ))
        }
    }
}

async fn insert_book<E, R>(
    State(state): State<AppState<R>>,
    Json(new_book): Json<NewBook>,
) -> Result<Json<Book>, (StatusCode, String)>
where
    E: Error,
    R: BookRepo<E>,
{
    let inserted_book = state
        .repo
        .insert_book(new_book)
        .await
        .map_err(internal_error)?;

    info!("Inserted book into the DB: {:?}", inserted_book);

    Ok(Json(inserted_book))
}

async fn update_book<E, R>(
    State(state): State<AppState<R>>,
    Path(id): Path<String>,
    Json(new_book): Json<NewBook>,
) -> Result<Json<Book>, (StatusCode, String)>
where
    E: Error,
    R: BookRepo<E>,
{
    let id = parse_book_id(id)?;

    let updated_book = state
        .repo
        .update_book(id, new_book)
        .await
        .map_err(internal_error)?;

    match updated_book {
        Some(book) => {
            info!("Updated book in DB: {:?}", book);
            Ok(Json(book))
        }
        None => {
            info!("Tried to update non-existent book with ID: {}", id);
            Err((
                StatusCode::NOT_FOUND,
                format!("No book found with ID: {}", id),
            ))
        }
    }
}

async fn delete_book<E, R>(State(state): State<AppState<R>>, Path(id): Path<String>) -> Response
where
    E: Error,
    R: BookRepo<E>,
{
    let deleted_or_error = try_to_delete_book(state.repo, id.clone()).await;

    match deleted_or_error {
        Ok(true) => {
            info!("Deleted book from DB with ID: {}", id);
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => {
            info!("Tried to delete non-existent book with ID: {}", id);
            (
                StatusCode::NOT_FOUND,
                format!("No book found with ID: {}", id),
            )
                .into_response()
        }
        Err(error_response) => error_response.into_response(),
    }
}

async fn try_to_delete_book<E: Error>(
    repo: impl BookRepo<E>,
    id: String,
) -> Result<bool, (StatusCode, String)> {
    let id = parse_book_id(id)?;
    repo.delete_book(id).await.map_err(internal_error)
}

/// Build a 500 response for an error
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn parse_book_id(id: String) -> Result<i32, (StatusCode, String)> {
    id.parse::<i32>()
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("Invalid book ID: {}", id)))
}
