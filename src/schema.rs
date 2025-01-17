// @generated automatically by Diesel CLI.

diesel::table! {
    books (id) {
        id -> Int4,
        name -> Varchar,
        author -> Varchar,
    }
}
