use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use crate::schema::{
    instances, ip_infos, licenses, objects, tags, tokens, unverified_users, users,
};

// diesel dosent like enums so we dont define these on db
#[derive(Deserialize, Clone, Copy)]
pub enum ObjectType {
    World = 0,
    Avatar = 1,
}

impl From<ObjectType> for &'static str {
    fn from(value: ObjectType) -> Self {
        match value {
            ObjectType::World => "worlds",
            ObjectType::Avatar => "avatars",
        }
    }
}

impl TryFrom<i16> for ObjectType {
    type Error = ();

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ObjectType::World),
            1 => Ok(ObjectType::Avatar),
            _ => Err(()),
        }
    }
}

pub enum PermissionsLevel {
    Default = 0,
    Moderator = 1,
    Admin = 2,
    Owner = 3,
    Internal = 4,
}

impl From<PermissionsLevel> for i16 {
    fn from(value: PermissionsLevel) -> Self {
        value as i16
    }
}

impl From<i16> for PermissionsLevel {
    fn from(value: i16) -> Self {
        match value {
            0 => PermissionsLevel::Default,
            1 => PermissionsLevel::Moderator,
            2 => PermissionsLevel::Admin,
            3 => PermissionsLevel::Owner,
            4 => PermissionsLevel::Internal,
            _ => PermissionsLevel::Default,
        }
    }
}

pub enum InstancePublicity {
    InviteOnly = 0,
    Friends = 1,
    FriendsOfFriends = 2,
    Public = 3,
}

impl From<InstancePublicity> for i16 {
    fn from(value: InstancePublicity) -> Self {
        value as i16
    }
}

impl From<i16> for InstancePublicity {
    fn from(value: i16) -> Self {
        match value {
            0 => InstancePublicity::InviteOnly,
            1 => InstancePublicity::Friends,
            2 => InstancePublicity::FriendsOfFriends,
            3 => InstancePublicity::Public,
            _ => InstancePublicity::InviteOnly,
        }
    }
}

pub enum ObjectPublicity {
    Private = 0,
    Friends = 1,
    Unlisted = 2,
    Public = 3,
}

impl From<ObjectPublicity> for i16 {
    fn from(value: ObjectPublicity) -> Self {
        value as i16
    }
}

impl From<i16> for ObjectPublicity {
    fn from(value: i16) -> Self {
        match value {
            0 => ObjectPublicity::Private,
            1 => ObjectPublicity::Friends,
            2 => ObjectPublicity::Unlisted,
            3 => ObjectPublicity::Public,
            _ => ObjectPublicity::Private,
        }
    }
}

#[derive(
    Queryable,
    Identifiable,
    Associations,
    Serialize,
    Selectable,
    Insertable,
    Debug,
    Clone,
    AsChangeset,
)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(User, foreign_key = creator))]
#[diesel(belongs_to(License, foreign_key = license))]
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
    pub likes: i32,
    pub dislikes: i32,
    pub publicity: i16,
    pub license: Uuid,
    pub encryption_key: Vec<u8>,
    pub encryption_iv: Vec<u8>,
    pub delete_at: Option<SystemTime>,
}

#[derive(Queryable, Selectable, Insertable, Serialize)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Instance, foreign_key = instance))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub permissions_level: i16,
    pub trust: i32,
    pub homeworld: Option<Uuid>,
    pub avatar: Option<Uuid>,
    pub instance: Option<Uuid>,
    pub identifier: Option<Vec<u8>>,
    pub delete_at: Option<SystemTime>,
    pub can_login: bool,
    pub upload_quota_used: i64,
    pub download_quota_used: i64,
}

#[derive(Queryable, Selectable, Insertable, Serialize)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UnverifiedUser {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub email: String,
    pub token: Vec<u8>,
    pub expires: SystemTime,
}

#[derive(Serialize, Queryable, Selectable, Debug)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PublicUserInfo {
    pub id: Uuid,
    pub username: String,
    pub homeworld: Option<Uuid>,
    pub avatar: Option<Uuid>,
    pub instance: Option<Uuid>,
}

impl From<User> for PublicUserInfo {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            username: value.username,
            homeworld: value.homeworld,
            avatar: value.avatar,
            instance: value.instance,
        }
    }
}

#[derive(Queryable, Selectable, Associations, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(User, foreign_key = user))]
pub struct Token {
    pub user: Uuid,
    pub token: Vec<u8>,
    pub renewable: bool,
    pub expires: SystemTime,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct License {
    pub id: Uuid,
    pub text: String,
}

#[derive(Queryable, Selectable, Associations, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Object, foreign_key = object))]
pub struct Tag {
    pub object: Uuid,
    pub tag: String,
}

#[derive(Queryable, Selectable, Associations, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Object, foreign_key = world))]
pub struct Instance {
    pub id: Uuid,
    pub server_token: Vec<u8>,
    pub world: Uuid,
    pub name: String,
    pub max_players: i16,
    pub publicity: i16,
    pub anyone_can_invite: bool,
    pub is_gameserver: bool,
    pub ip: ipnet::IpNet,
    pub port: i32,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IpInfo {
    pub ip: ipnet::IpNet,
    pub accounts_created: i16,
    pub account_creation_count_reset: SystemTime,
    pub login_attempts: i16,
    pub login_attempts_reset: SystemTime,
}
