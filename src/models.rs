use std::time::SystemTime;

use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::{tokens, users};

// diesel dosent like enums so we dont define these on db
pub enum PermissionLevel {
    None = 0,
    Creator = 1,
    Moderator = 2,
    Admin = 3,
}

pub enum ObjectType {
    World = 0,
    Avatar = 1,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(treat_none_as_default_value = false)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password: Vec<u8>,
    pub salt: Vec<u8>,
    pub email: String,
    pub verified_email: bool,
    pub permisions: i16, // PermisionLevel
    pub trust: i32,
    pub homeworld: Option<Uuid>,
    pub avatar: Option<Uuid>,
}
#[derive(Serialize, Queryable, Selectable, Debug)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(treat_none_as_default_value = false)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
    pub homeworld: Option<Uuid>,
    pub avatar: Option<Uuid>,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(treat_none_as_default_value = false)]
pub struct Token {
    pub user: Uuid,
    pub token: Vec<u8>,
    pub expiry: Option<SystemTime>,
    pub renewable: bool,
}
