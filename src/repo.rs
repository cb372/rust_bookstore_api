use crate::models::{Book, NewBook};
use std::error::Error;
use std::future::Future;

pub trait BookRepo<E: Error> {
    fn list_books(&self) -> impl Future<Output = Result<Vec<Book>, E>> + Send;

    fn get_book(&self, id: i32) -> impl Future<Output = Result<Option<Book>, E>> + Send;

    fn insert_book(&mut self, new_book: NewBook) -> impl Future<Output = Result<Book, E>> + Send;

    fn update_book(
        &mut self,
        id: i32,
        new_book: NewBook,
    ) -> impl Future<Output = Result<Option<Book>, E>> + Send;

    /// Returns true if the book existed and was deleted, false otherwise
    fn delete_book(&mut self, id: i32) -> impl Future<Output = Result<bool, E>> + Send;
}
