use crate::schema::books;

#[derive(Debug, serde::Serialize, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = books)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Book {
    id: i32,
    name: String,
    author: String
}

// TODO could build this using a macro, as it is just Book minus the ID field
#[derive(serde::Deserialize, diesel::Insertable, diesel::AsChangeset)]
#[diesel(table_name = books)]
pub struct NewBook {
    name: String,
    author: String
}
