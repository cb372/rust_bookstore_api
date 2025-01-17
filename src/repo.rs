use crate::models::{Book, NewBook};
use std::error::Error;

pub trait BookRepo<E: Error> {
    async fn list_books(&self) -> Result<Vec<Book>, E>;

    async fn get_book(&self, id: i32) -> Result<Option<Book>, E>;

    async fn insert_book(&self, new_book: NewBook) -> Result<Book, E>;

    async fn update_book(
        &self,
        id: i32,
        new_book: NewBook,
    ) -> Result<Option<Book>, E>;

    /// Returns true if the book existed and was deleted, false otherwise
    async fn delete_book(&self, id: i32) -> Result<bool, E>;
}
