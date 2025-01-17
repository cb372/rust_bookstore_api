use std::error::Error;
use std::fmt;

use crate::models::{Book, NewBook};
use crate::repo::{BookRepo, BookRepoError};
use crate::schema::books;
use bb8::Pool;
use diesel::{OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::{
    pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection, RunQueryDsl,
};

pub type DBPool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

pub async fn create_db_pool() -> DBPool {
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or("postgres://localhost/bookstore".to_string());
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
    Pool::builder()
        .build(config)
        .await
        .expect("Failed to create DB connection pool")
}

#[derive(Debug)]
pub enum DatabaseError {
    PoolError(bb8::RunError<diesel_async::pooled_connection::PoolError>),
    ResultError(diesel::result::Error),
}

impl From<bb8::RunError<diesel_async::pooled_connection::PoolError>>
    for BookRepoError<DatabaseError>
{
    fn from(error: bb8::RunError<diesel_async::pooled_connection::PoolError>) -> Self {
        BookRepoError::new(DatabaseError::PoolError(error))
    }
}

impl From<diesel::result::Error> for BookRepoError<DatabaseError> {
    fn from(error: diesel::result::Error) -> Self {
        BookRepoError::new(DatabaseError::ResultError(error))
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::PoolError(_) => {
                write!(f, "problem getting a connection from the connection pool")
            }
            DatabaseError::ResultError(_) => {
                write!(f, "problem executing a statement against the DB")
            }
        }
    }
}

impl Error for DatabaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DatabaseError::PoolError(e) => Some(e),
            DatabaseError::ResultError(e) => Some(e),
        }
    }
}

#[derive(Clone)]
pub struct DatabaseBookRepo {
    pool: DBPool,
}

impl DatabaseBookRepo {
    pub fn new(pool: DBPool) -> Self {
        DatabaseBookRepo { pool }
    }
}

impl BookRepo<DatabaseError> for DatabaseBookRepo {
    async fn list_books(&self) -> Result<Vec<Book>, BookRepoError<DatabaseError>> {
        let mut conn = self.pool.get().await?;

        let books = books::table
            .select(Book::as_select())
            .limit(100)
            .load(&mut conn)
            .await?;

        Ok(books)
    }

    async fn get_book(&self, id: i32) -> Result<Option<Book>, BookRepoError<DatabaseError>> {
        let mut conn = self.pool.get().await?;

        let maybe_book = books::table
            .find(id)
            .select(Book::as_select())
            .first(&mut conn)
            .await
            .optional()?;

        Ok(maybe_book)
    }

    async fn insert_book(&self, new_book: NewBook) -> Result<Book, BookRepoError<DatabaseError>> {
        let mut conn = self.pool.get().await?;

        let inserted_book = diesel::insert_into(books::table)
            .values(new_book)
            .returning(Book::as_returning())
            .get_result(&mut conn)
            .await?;

        Ok(inserted_book)
    }

    async fn update_book(
        &self,
        id: i32,
        new_book: NewBook,
    ) -> Result<Option<Book>, BookRepoError<DatabaseError>> {
        let mut conn = self.pool.get().await?;

        let updated_book = diesel::update(books::table.find(id))
            .set(new_book)
            .returning(Book::as_returning())
            .get_result(&mut conn)
            .await
            .optional()?;

        Ok(updated_book)
    }

    async fn delete_book(&self, id: i32) -> Result<bool, BookRepoError<DatabaseError>> {
        let mut conn = self.pool.get().await?;

        let deleted = diesel::delete(books::table.find(id))
            .execute(&mut conn)
            .await
            .map(|affected_rows| affected_rows == 1)?;

        Ok(deleted)
    }
}
