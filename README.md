# Bookstore API

This is an example of a simple REST API backed by a Postgres DB, implemented in
Rust.

## Functionality

The API offers the usual CRUD endpoints:
* create a book
* list books
* get a book by ID
* update a book
* delete a book

## Tech stack

* `axum` for the HTTP API
* `diesel` + `diesel-async` for the ORM/Postgres integration and DB migrations
* `bb8` for the DB connection pool

Everything is built on Tokio and runs asynchronously.

The integration tests use `testcontainers` to spin up Postgres in a Docker
container, and `reqwest` as the HTTP client.

## Architecture

There is a `BookRepo` trait defined in `repo.rs`, to abstract away the details
of talking to Postgres.

The `axum` HTTP routes and handlers are defined in `api.rs`. The handlers take
an `impl BookRepo` as a dependency, so they are decoupled from the DB and can be
unit-tested against a fake in-memory repository.

## To run the app locally

Start Postgres locally, or in a container or whatever.

Set the `DATABASE_URL` environment variable, e.g. `postgres://localhost/bookstore`.

Install the [Diesel
CLI](https://diesel.rs/guides/getting-started.html#installing-diesel-cli).

Run `diesel setup` to create the database in Postgres.

Run `diesel migrate` to run the DB migrations, which creates the `books` table.

Run `cargo run` to start the HTTP server.

Now you should be able to hit `localhost:3000`:

```
$ curl localhost:3000/books
[]
```
