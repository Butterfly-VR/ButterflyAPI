use crate::ApiError;
use crate::AppState;
use crate::auth;
use crate::models::ObjectType;
use crate::models::User;
use crate::schema::objects;
use crate::schema::users;
use axum::Extension;
use axum::extract::State;
use axum::middleware;
use axum::{Json, Router, routing::get};
use diesel::prelude::*;
use diesel::update;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

const USER_API_ROUTE: &str = "/this_user";
const USER_HOMEWORLD_ROUTE: &str = constcat::concat!(USER_API_ROUTE, "/homeworld");
const USER_AVATAR_ROUTE: &str = constcat::concat!(USER_API_ROUTE, "/avatar");

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<Uuid>,
) -> Result<Json<User>, ApiError> {
    let mut conn = state.pool.get().await?;

    let mut user = users::table
        .select(User::as_select())
        .filter(users::id.eq(user))
        .first(&mut conn)
        .await?;

    // homeworld and avatar need to be nullable to avoid circular dependacies
    // but we want to always return something valid
    // these values should corrospond to objects uploaded by the ButterflyDev account
    user.homeworld = Some(user.homeworld.unwrap_or(Uuid::nil()));
    user.avatar = Some(user.avatar.unwrap_or(Uuid::from_u64_pair(0, 1)));
    return Ok(Json(user));
}

#[derive(Serialize, Deserialize)]
pub struct Homeworld {
    uuid: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct Avatar {
    uuid: Uuid,
}

pub async fn get_homeworld(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<Uuid>,
) -> Result<Json<Homeworld>, ApiError> {
    let mut conn = state.pool.get().await?;

    let homeworld = users::table
        .select(users::homeworld)
        .filter(users::id.eq(user))
        .first::<Option<Uuid>>(&mut conn)
        .await?;

    // homeworld and avatar need to be nullable to avoid circular dependacies
    // but we want to always return something valid
    // these values should corrospond to objects uploaded by the ButterflyDev account
    let homeworld = homeworld.unwrap_or(Uuid::nil());
    return Ok(Json(Homeworld { uuid: homeworld }));
}

pub async fn get_avatar(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<Uuid>,
) -> Result<Json<Avatar>, ApiError> {
    let mut conn = state.pool.get().await?;

    let avatar = users::table
        .select(users::avatar)
        .filter(users::id.eq(user))
        .first::<Option<Uuid>>(&mut conn)
        .await?;

    // homeworld and avatar need to be nullable to avoid circular dependacies
    // but we want to always return something valid
    // these values should corrospond to objects uploaded by the ButterflyDev account
    let avatar = avatar.unwrap_or(Uuid::from_u64_pair(0, 1));
    return Ok(Json(Avatar { uuid: avatar }));
}

pub async fn set_homeworld(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<Uuid>,
    Json(homeworld): Json<Homeworld>,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;

    let homeworld = objects::table
        .select(objects::id)
        .filter(objects::id.eq(homeworld.uuid))
        .filter(objects::object_type.eq(ObjectType::World as i16))
        .first::<Uuid>(&mut conn)
        .await?;

    update(users::table)
        .filter(users::id.eq(user))
        .set(users::homeworld.eq(homeworld))
        .execute(&mut conn)
        .await?;
    Ok(())
}

pub async fn set_avatar(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<Uuid>,
    Json(avatar): Json<Avatar>,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;

    let avatar = objects::table
        .select(objects::id)
        .filter(objects::id.eq(avatar.uuid))
        .filter(objects::object_type.eq(ObjectType::Avatar as i16))
        .first::<Uuid>(&mut conn)
        .await?;

    update(users::table)
        .filter(users::id.eq(user))
        .set(users::avatar.eq(avatar))
        .execute(&mut conn)
        .await?;
    Ok(())
}

pub fn user_api_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(USER_API_ROUTE, get(get_user))
        .route(USER_HOMEWORLD_ROUTE, get(get_homeworld).post(set_homeworld))
        .route(USER_AVATAR_ROUTE, get(get_avatar).post(set_avatar))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_auth,
        ))
        .with_state(app_state)
}
