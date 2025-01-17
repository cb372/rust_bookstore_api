use crate::models::{Book, NewBook};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct BookRepoError<E: Error> {
    source: E,
}

impl<E: Error> BookRepoError<E> {
    pub fn new(source: E) -> Self {
        BookRepoError { source }
    }
}

impl<E: Error> From<E> for BookRepoError<E> {
    fn from(source: E) -> Self {
        BookRepoError { source }
    }
}

impl<E: Error> fmt::Display for BookRepoError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, f)
    }
}

impl<E: Error> Error for BookRepoError<E> {}

pub trait BookRepo<E: Error> {
    async fn list_books(&self) -> Result<Vec<Book>, BookRepoError<E>>;

    async fn get_book(&self, id: i32) -> Result<Option<Book>, BookRepoError<E>>;

    async fn insert_book(&self, new_book: NewBook) -> Result<Book, BookRepoError<E>>;

    async fn update_book(
        &self,
        id: i32,
        new_book: NewBook,
    ) -> Result<Option<Book>, BookRepoError<E>>;

    /// Returns true if the book existed and was deleted, false otherwise
    async fn delete_book(&self, id: i32) -> Result<bool, BookRepoError<E>>;
}
