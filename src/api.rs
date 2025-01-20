use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::error::Error;
use tracing::info;

use crate::models::{Book, NewBook};
use crate::repo::BookRepo;

#[derive(Clone)]
struct AppState<R> {
    repo: R,
}

pub fn build_api<E: Error + 'static>(
    repo: impl BookRepo<E> + Send + Sync + Clone + 'static,
) -> Router {
    Router::new()
        .route("/books", get(list_books).post(insert_book))
        .route(
            "/books/{id}",
            get(get_book).put(update_book).delete(delete_book),
        )
        .with_state(AppState { repo })
}

async fn list_books<E, R>(
    State(state): State<AppState<R>>,
) -> Result<Json<Vec<Book>>, (StatusCode, String)>
where
    E: Error,
    R: BookRepo<E> + Send + Sync + Clone,
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
    State(mut state): State<AppState<R>>,
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
    State(mut state): State<AppState<R>>,
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
    mut repo: impl BookRepo<E>,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fmt::Display;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Debug)]
    struct MockError {}

    impl Display for MockError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("something went wrong!")
        }
    }

    impl Error for MockError {}

    #[derive(Clone)]
    struct MockBookRepo {
        db: Arc<Mutex<HashMap<i32, Book>>>,
        raise_errors: bool,
    }

    impl BookRepo<MockError> for MockBookRepo {
        async fn list_books(&self) -> Result<Vec<Book>, MockError> {
            if self.raise_errors {
                Err(MockError {})
            } else {
                let db = self.db.lock().unwrap();
                Ok(db.values().cloned().collect())
            }
        }

        async fn get_book(&self, id: i32) -> Result<Option<Book>, MockError> {
            if self.raise_errors {
                Err(MockError {})
            } else {
                let db = self.db.lock().unwrap();
                Ok(db.get(&id).cloned())
            }
        }

        async fn insert_book(&mut self, new_book: NewBook) -> Result<Book, MockError> {
            if self.raise_errors {
                Err(MockError {})
            } else {
                let mut db = self.db.lock().unwrap();
                let fresh_id = db.keys().max().unwrap_or(&0) + 1;
                let book = Book {
                    id: fresh_id,
                    name: new_book.name,
                    author: new_book.author,
                };
                db.insert(fresh_id, book.clone());
                Ok(book)
            }
        }

        async fn update_book(
            &mut self,
            _id: i32,
            _new_book: NewBook,
        ) -> Result<Option<Book>, MockError> {
            todo!()
        }

        async fn delete_book(&mut self, _id: i32) -> Result<bool, MockError> {
            todo!()
        }
    }

    impl Display for MockBookRepo {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "MockBookRepo with DB: {:?}", self.db)
        }
    }

    fn build_db() -> Arc<Mutex<HashMap<i32, Book>>> {
        let mut db = HashMap::new();
        db.insert(
            10,
            Book {
                id: 10,
                name: "TAOCP".to_string(),
                author: "Donald Knuth".to_string(),
            },
        );
        db.insert(
            20,
            Book {
                id: 20,
                name: "Manual of Ethics".to_string(),
                author: "John Mackenzie".to_string(),
            },
        );
        Arc::new(Mutex::new(db))
    }

    #[tokio::test]
    async fn list_books_returns_list_of_books_in_an_unspecified_order() {
        let db = build_db();
        let repo = MockBookRepo {
            db: db.clone(),
            raise_errors: false,
        };
        let state = State(AppState { repo });

        let Json(mut result) = list_books(state).await.unwrap();
        result.sort_by(|a, b| a.id.cmp(&b.id));

        let mut db_values = db.lock().unwrap().values().cloned().collect::<Vec<Book>>();
        db_values.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(result, db_values);
    }

    #[tokio::test]
    async fn list_books_returns_a_500_response_if_repo_raises_an_error() {
        let repo = MockBookRepo {
            db: build_db(),
            raise_errors: true,
        };
        let state = State(AppState { repo });

        let (status_code, _) = list_books(state)
            .await
            .expect_err("Expected a 500 response");

        assert_eq!(status_code, 500);
    }

    #[tokio::test]
    async fn get_book_returns_a_book_if_it_exists_in_repo() {
        let repo = MockBookRepo {
            db: build_db(),
            raise_errors: false,
        };
        let state = State(AppState { repo });
        let path = Path("10".to_string());

        let Json(result) = get_book(state, path).await.unwrap();

        assert_eq!(result.id, 10);
        assert_eq!(result.name, "TAOCP");
        assert_eq!(result.author, "Donald Knuth");
    }

    #[tokio::test]
    async fn get_book_returns_a_404_response_if_book_is_not_found() {
        let repo = MockBookRepo {
            db: build_db(),
            raise_errors: false,
        };
        let state = State(AppState { repo });
        let path = Path("99".to_string());

        let (status_code, _) = get_book(state, path)
            .await
            .expect_err("Expected a 404 response");

        assert_eq!(status_code, 404);
    }

    #[tokio::test]
    async fn get_book_returns_a_500_response_if_repo_raises_an_error() {
        let repo = MockBookRepo {
            db: build_db(),
            raise_errors: true,
        };
        let state = State(AppState { repo });
        let path = Path("99".to_string());

        let (status_code, _) = get_book(state, path)
            .await
            .expect_err("Expected a 500 response");

        assert_eq!(status_code, 500);
    }

    #[tokio::test]
    async fn insert_book_inserts_a_book_into_repo_and_returns_the_inserted_book() {
        let db = build_db();
        let repo = MockBookRepo {
            db: db.clone(),
            raise_errors: false,
        };
        let state = State(AppState { repo });
        let new_book = NewBook {
            name: "Paradise Lost".to_string(),
            author: "John Milton".to_string(),
        };
        let new_book_json = Json(new_book.clone());

        let Json(inserted_book) = insert_book(state, new_book_json).await.unwrap();

        assert_eq!(inserted_book.name, new_book.name);
        assert_eq!(inserted_book.author, new_book.author);

        // Verify that the book has been inserted into the DB
        let updated_db = db.lock().unwrap();
        assert_eq!(updated_db.get(&inserted_book.id), Some(&inserted_book));
    }

    #[tokio::test]
    async fn insert_book_returns_a_500_response_if_repo_raises_an_error() {
        let repo = MockBookRepo {
            db: build_db(),
            raise_errors: true,
        };
        let state = State(AppState { repo });
        let new_book = NewBook {
            name: "Paradise Lost".to_string(),
            author: "John Milton".to_string(),
        };
        let new_book_json = Json(new_book.clone());

        let (status_code, _) = insert_book(state, new_book_json)
            .await
            .expect_err("Expected a 500 response");

        assert_eq!(status_code, 500);
    }

    // TODO skipped the tests for updating and deleting
}
