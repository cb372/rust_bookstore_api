use diesel::prelude::*;
use diesel_migrations::*;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::{ContainerAsync, runners::AsyncRunner};
use tokio::time::{sleep, Duration};

use rust_bookstore_api::start_server;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

// Note: not reusing the application's models is a deliberate choice
#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
struct Book {
    id: i32,
    name: String,
    author: String,
}
#[derive(Debug, serde::Serialize)]
struct BookInput {
    name: String,
    author: String,
}

struct BookClient {
    client: reqwest::Client
}

impl BookClient {
    async fn list_books(&self) -> Result<Vec<Book>, reqwest::Error> {
        self.client
            .get("http://localhost:3000/books")
            .send()
            .await?
            .json::<Vec<Book>>()
            .await
    }

    async fn get_book_raw(&self, id: i32) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .get(format!("http://localhost:3000/books/{id}"))
            .send()
            .await
    }

    async fn get_book(&self, id: i32) -> Result<Book, reqwest::Error> {
        self.get_book_raw(id)
            .await?
            .json::<Book>()
            .await
    }

    async fn insert_book(&self, name: String, author: String) -> Result<Book, reqwest::Error> {
        let input = BookInput { name, author };
        self.client
            .post("http://localhost:3000/books")
            .json(&input)
            .send()
            .await?
            .json::<Book>()
            .await
    }

    async fn update_book_raw(&self, id: i32, name: String, author: String) -> Result<reqwest::Response, reqwest::Error> {
        let input = BookInput { name, author };
        self.client
            .put(format!("http://localhost:3000/books/{id}"))
            .json(&input)
            .send()
            .await
    }

    async fn update_book(&self, id: i32, name: String, author: String) -> Result<Book, reqwest::Error> {
        self.update_book_raw(id, name, author)
            .await?
            .json::<Book>()
            .await
    }

    async fn delete_book(&self, id: i32) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .delete(format!("http://localhost:3000/books/{id}"))
            .send()
            .await
    }
}

async fn setup_database(container: &ContainerAsync<Postgres>) -> String {
    let connection_string = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        container.get_host_port_ipv4(5432).await.unwrap()
    );

    print!("Giving Postgres a few seconds to startup... ");
    sleep(Duration::from_secs(3)).await;
    println!("Done");

    let mut connection = PgConnection::establish(&connection_string)
        .unwrap_or_else(|_| panic!("Error connecting to {}", connection_string));

    println!("Running DB migrations...");
    let migration_versions = connection.run_pending_migrations(MIGRATIONS).unwrap();
    println!("Executed {} migrations", migration_versions.len());

    connection_string
}

async fn run_tests(client: BookClient) -> Result<(), reqwest::Error> {
    // Start with an empty book database
    let books = client.list_books().await?;
    assert_eq!(0, books.len());

    // Add a couple of books
    let book1 = client.insert_book("Great Expectations".to_string(), "Charles Dickens".to_string()).await?;
    assert_eq!("Great Expectations".to_string(), book1.name);
    assert_eq!("Charles Dickens".to_string(), book1.author);

    let book2 = client.insert_book("Never Let Me Go".to_string(), "Kazuo Ishiguro".to_string()).await?;
    assert_eq!("Never Let Me Go".to_string(), book2.name);
    assert_eq!("Kazuo Ishiguro".to_string(), book2.author);

    // List the books again - there should be 2 now
    let books = client.list_books().await?;
    assert_eq!(2, books.len());

    // Retrieve the books we just inserted
    let retrieved_book = client.get_book(book1.id).await?;
    assert_eq!(retrieved_book, book1);
    let retrieved_book = client.get_book(book2.id).await?;
    assert_eq!(retrieved_book, book2);

    // Retrieve a non-existent book
    let get_book_response = client.get_book_raw(99).await?;
    assert_eq!(404, get_book_response.status().as_u16());

    // Update one of the books
    let updated_book = client.update_book(book2.id, "The Unconsoled".to_string(), "Kazuo Ishiguro".to_string()).await?;
    assert_eq!(book2.id, updated_book.id);
    assert_eq!("The Unconsoled".to_string(), updated_book.name);
    assert_eq!("Kazuo Ishiguro".to_string(), updated_book.author);

    // Retrieve the updated book
    let retrieved_book = client.get_book(book2.id).await?;
    assert_eq!(updated_book, retrieved_book);

    // Update a non-existent book -> get a 404 response
    let update_book_response = client.update_book_raw(99, "foo".to_string(), "bar".to_string()).await?;
    assert_eq!(404, update_book_response.status().as_u16());

    // Delete a book
    let delete_book_response = client.delete_book(book2.id).await?;
    assert_eq!(204, delete_book_response.status().as_u16());

    // Check that the book has been deleted
    let books = client.list_books().await?;
    assert_eq!(1, books.len());

    let get_book_response = client.get_book_raw(book2.id).await?;
    assert_eq!(404, get_book_response.status().as_u16());

    // The other book we inserted should still exist
    let book1_again = client.get_book(book1.id).await?;
    assert_eq!(book1, book1_again);

    // Delete a non-existent book -> get a 404 response
    let delete_book_response = client.delete_book(99).await?;
    assert_eq!(404, delete_book_response.status().as_u16());

    Ok(())
}

#[tokio::test]
async fn bookstore_api_integration_test() {
    // Start Postgres in a Docker container and run the DB migrations
    let postgres = Postgres::default().start().await.unwrap();
    let db_url = setup_database(&postgres).await;

    // Run the HTTP server in a background thread, so we can run tests against it
    let server = start_server(db_url).await;
    tokio::spawn(async move {
        server.await.unwrap();
    });

    let client = BookClient { client: reqwest::Client::new() };

    run_tests(client).await.unwrap();
}
