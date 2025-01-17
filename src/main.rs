pub mod models;
pub mod schema;

use axum::{
    http::StatusCode,
    extract::{Path, State},
    response::{Response, IntoResponse},
    routing::get,
    Json,
    Router
};
use tokio::net::TcpListener;
use diesel::{OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection, RunQueryDsl,
};
use bb8::Pool;
use tracing::info;
use models::{Book, NewBook};
use schema::books;

type DBPool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_pool = create_db_pool().await;

    // TODO use the db_pool to create a repository

    let app = Router::new()
        .route("/books", get(list_books).post(insert_book))
        .route("/books/{id}", get(get_book).put(update_book).delete(delete_book))
        .with_state(db_pool);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    info!("Listening on {}", local_addr);

    axum::serve(listener, app).await.unwrap();
}

async fn create_db_pool() -> DBPool {
    let db_url = std::env::var("DATABASE_URL").unwrap_or("postgres://localhost/bookstore".to_string());
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
    Pool::builder().build(config).await.expect("Failed to create DB connection pool")
}

async fn list_books(
    State(pool): State<DBPool>
) -> Result<Json<Vec<Book>>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(internal_error)?;

    // TODO pagination

    let results = books::table
        .select(Book::as_select())
        .limit(100)
        .load(&mut conn)
        .await
        .map_err(internal_error)?;

    info!("Retrieved {} books from the DB", results.len());

    Ok(Json(results))
}

async fn get_book(
    State(pool): State<DBPool>,
    Path(id): Path<String>
) -> Result<Json<Book>, (StatusCode, String)> {
    let id = parse_book_id(id)?;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let book = books::table
        .find(id)
        .select(Book::as_select())
        .first(&mut conn)
        .await
        .optional()
        .map_err(internal_error)?;

    match book {
        Some(book) => {
            info!("Retrieved book from DB: {:?}", book);
            Ok(Json(book))
        },
        None => {
            info!("No book found in DB with ID: {}", id);
            Err((StatusCode::NOT_FOUND, format!("No book found with ID: {}", id)))
        }
    }
}

async fn insert_book(
    State(pool): State<DBPool>,
    Json(new_book): Json<NewBook>
) -> Result<Json<Book>, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(internal_error)?;

    let inserted_book = diesel::insert_into(books::table)
        .values(new_book)
        .returning(Book::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(internal_error)?;

    info!("Inserted book into the DB: {:?}", inserted_book);

    Ok(Json(inserted_book))
}

async fn update_book(
    State(pool): State<DBPool>,
    Path(id): Path<String>,
    Json(new_book): Json<NewBook>
) -> Result<Json<Book>, (StatusCode, String)> {
    let id = parse_book_id(id)?;

    let mut conn = pool.get().await.map_err(internal_error)?;

    let updated_book = diesel::update(books::table.find(id))
        .set(new_book)
        .returning(Book::as_returning())
        .get_result(&mut conn)
        .await
        .optional()
        .map_err(internal_error)?;

    match updated_book {
        Some(book) => {
            info!("Updated book in DB: {:?}", book);
            Ok(Json(book))
        },
        None => {
            info!("Tried to update non-existent book with ID: {}", id);
            Err((StatusCode::NOT_FOUND, format!("No book found with ID: {}", id)))
        }
    }
}

async fn delete_book(
    State(pool): State<DBPool>,
    Path(id): Path<String>
) -> Response {
    let affected_rows_or_error = try_to_delete_book(pool, id.clone()).await;

    match affected_rows_or_error {
        Ok(0) => {
            info!("Tried to delete non-existent book with ID: {}", id);
            (StatusCode::NOT_FOUND, format!("No book found with ID: {}", id)).into_response()
        },
        Ok(_) => {
            info!("Deleted book from DB with ID: {}", id);
            StatusCode::NO_CONTENT.into_response()
        },
        Err(error_response) => error_response.into_response()
    }
}

async fn try_to_delete_book(
    pool: DBPool,
    id: String
) -> Result<usize, (StatusCode, String)> {
    let id = parse_book_id(id)?;

    let mut conn = pool.get().await.map_err(internal_error)?;

    diesel::delete(books::table.find(id))
        .execute(&mut conn)
        .await
        .map_err(internal_error)
}

/// Build a 500 response for an error
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn parse_book_id(id: String) -> Result<i32, (StatusCode, String)> {
    id
        .parse::<i32>()
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("Invalid book ID: {}", id)))
}
