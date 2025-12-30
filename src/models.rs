use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use crate::schema::*;

// diesel dosent like enums so we dont define these on db
#[derive(Deserialize, Clone, Copy)]
pub enum ObjectType {
    World = 0,
    Avatar = 1,
}

#[derive(Queryable, Identifiable, Serialize, Selectable, Insertable, Debug, Clone, AsChangeset)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Object {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub flags: Vec<Option<bool>>,
    pub updated_at: SystemTime,
    pub created_at: SystemTime,
    pub verified: bool,
    pub object_size: i64,
    pub image_size: i64,
    pub creator: Uuid,
    pub object_type: i16,
    pub publicity: i16,
    pub license: i32,
    pub encryption_key: Vec<u8>,
    pub encryption_iv: Vec<u8>,
}

#[derive(Queryable, Selectable, Insertable, Serialize)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password: Vec<u8>,
    pub salt: Vec<u8>,
    pub email: String,
    pub permisions: Vec<Option<bool>>,
    pub trust: i32,
    pub homeworld: Option<Uuid>,
    pub avatar: Option<Uuid>,
}

#[derive(Queryable, Selectable, Insertable, Serialize)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UnverifiedUser {
    pub id: Uuid,
    pub username: String,
    pub password: Vec<u8>,
    pub salt: Vec<u8>,
    pub email: String,
    pub token: Vec<u8>,
    pub expiry: SystemTime,
}

#[derive(Serialize, Queryable, Selectable, Debug)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PublicUserInfo {
    pub id: Uuid,
    pub username: String,
    pub homeworld: Option<Uuid>,
    pub avatar: Option<Uuid>,
}

impl From<User> for PublicUserInfo {
    fn from(value: User) -> Self {
        PublicUserInfo {
            id: value.id,
            username: value.username,
            homeworld: value.homeworld,
            avatar: value.avatar,
        }
    }
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Token {
    pub user: Uuid,
    pub token: Vec<u8>,
    pub expiry: SystemTime,
    pub renewable: bool,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct License {
    license: i32,
    text: String,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Tag {
    object: Uuid,
    tag: String,
}
