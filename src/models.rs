use crate::schema::books;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, diesel::Queryable, diesel::Selectable)]
#[diesel(table_name = books)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Book {
    pub id: i32,
    pub name: String,
    pub author: String,
}

// TODO could build this using a macro, as it is just Book minus the ID field
#[derive(Clone, serde::Deserialize, diesel::Insertable, diesel::AsChangeset)]
#[diesel(table_name = books)]
pub struct NewBook {
    pub name: String,
    pub author: String,
}
