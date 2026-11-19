use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, IntoStaticStr};
use uuid::Uuid;

use crate::schema::*;

#[derive(Deserialize, PartialEq, Debug, Clone, Copy, EnumString, AsRefStr, IntoStaticStr)]
pub enum ObjectType {
    World,
    Avatar,
}

#[derive(Deserialize, PartialEq, Debug, Clone, Copy, EnumString, AsRefStr, IntoStaticStr)]
pub enum PermissionsLevel {
    Default,
    Moderator,
    Admin,
}

#[derive(Deserialize, PartialEq, Debug, Clone, Copy, EnumString, AsRefStr, IntoStaticStr)]
pub enum InstancePublicity {
    InviteOnly,
    Friends,
    FriendsOfFriends,
    Public,
}

#[derive(Deserialize, PartialEq, Debug, Clone, Copy, EnumString, AsRefStr, IntoStaticStr)]
pub enum ObjectPublicity {
    Private,
    Friends,
    Unlisted,
    Public,
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
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub verified: bool,
    pub object_size: i64,
    pub image_size: i64,
    pub creator: Uuid,
    pub object_type: String,
    pub likes: i32,
    pub dislikes: i32,
    pub publicity: String,
    pub license: Uuid,
    pub encryption_key: Vec<u8>,
    pub encryption_iv: Vec<u8>,
    pub delete_at: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable, Insertable, Serialize)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Instance, foreign_key = instance))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub email: String,
    #[serde(skip_serializing)]
    pub password: Vec<u8>,
    #[serde(skip_serializing)]
    pub salt: Vec<u8>,
    pub permissions_level: String,
    pub trust: i32,
    pub homeworld: Option<Uuid>,
    pub avatar: Option<Uuid>,
    pub instance: Option<Uuid>,
    pub identifier: Option<Vec<u8>>,
    pub delete_at: Option<DateTime<Utc>>,
    pub can_login: bool,
    pub upload_quota_used: i64,
    pub download_quota_used: i64,
    pub status: String,
    pub profile_dark_mode: bool,
    pub profile_style: String,
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
    pub expires: DateTime<Utc>,
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
    pub expires: DateTime<Utc>,
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

// todo: track instance creator to handle friends only publicity correctly
// todo: what happens when instance creator leaves?
#[derive(Queryable, Selectable, Associations, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Object, foreign_key = world))]
pub struct Instance {
    pub id: Uuid,
    pub server_token: Vec<u8>,
    pub world: Uuid,
    pub name: String,
    pub max_players: i16,
    pub publicity: String,
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
    pub account_creation_count_reset: DateTime<Utc>,
    pub login_attempts: i16,
    pub login_attempts_reset: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ObjectDownloadToken {
    pub token: Uuid,
    pub object_id: Uuid,
    pub current_step: i16,
    pub expiry: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Insertable, Associations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(User, foreign_key = target))]
pub struct Moderation {
    pub id: Uuid,
    pub target: Uuid,
    pub moderator: Option<Uuid>,
    pub type_: String,
    pub expires: Option<DateTime<Utc>>,
    pub details: Option<String>,
}
