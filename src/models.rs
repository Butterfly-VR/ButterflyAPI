use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::users;

#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password: Vec<u8>,
    pub salt: Vec<u8>,
    pub email: String,
}
#[derive(Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a [u8],
    pub salt: &'a [u8],
    pub email: &'a str,
}
#[derive(Serialize, Queryable, Selectable, Debug)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
}
