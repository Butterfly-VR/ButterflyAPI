use diesel::prelude::*;
use serde::Serialize;

use crate::schema::temp_test_users;

#[derive(Queryable, Selectable)]
#[diesel(table_name = temp_test_users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: Vec<u8>,
    pub salt: Vec<u8>,
    pub email: String,
}
#[derive(Insertable)]
#[diesel(table_name = temp_test_users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser<'a> {
    pub username: &'a str,
    pub password: &'a [u8],
    pub salt: &'a [u8],
    pub email: &'a str,
}
#[derive(Serialize, Queryable, Selectable, Debug)]
#[diesel(table_name = temp_test_users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PublicUser {
    pub id: i32,
    pub username: String,
}
