use crate::ApiResponse;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::HASHER_ALGORITHM;
use crate::HASHER_PARAMETERS;
use crate::HASHER_VERSION;
use crate::ROUTE_ORIGIN;
use crate::auth;
use crate::get_conn_async;
use crate::models::*;
use crate::schema::users::dsl::*;
use argon2::Argon2;
use axum::extract::Path;
use axum::extract::State;
use axum::http;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use password_hash::rand_core;
use password_hash::rand_core::RngCore;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

const USERS_ROUTE: &str = constcat::concat!(ROUTE_ORIGIN, "/user");
const USER_ID_ROUTE: &str = constcat::concat!(USERS_ROUTE, "/{usr_id}");

#[derive(Deserialize)]
struct SignUpRequest {
    pub username: String,
    pub password_hash: Vec<u8>,
    pub email: String,
}

pub async fn sign_up(
    State(state): State<Arc<AppState>>,
    Json(json): Json<SignUpRequest>,
) -> ApiResponse<()> {
    let mut db_connection = get_conn_async(&state.pool).await;

    if users
        .count()
        .filter(username.eq(&json.username))
        .or_filter(email.eq(&json.email))
        .get_result(&mut db_connection)
        .unwrap_or(1)
        != 0
    {
        return ApiResponse::Bad(
            http::StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                code: ErrorCode::UserExists,
                message: Some(String::from("Username or email already in use.")),
            }),
        );
    }

    let mut password_salt = [0; 64];
    if rand_core::OsRng.try_fill_bytes(&mut password_salt).is_err() {
        return ApiResponse::BadNoInfo(StatusCode::SERVICE_UNAVAILABLE);
    }

    let mut block = None;
    for lock in state.hasher_memory.iter() {
        if let Ok(b) = lock.try_lock() {
            block = Some(b);
            break;
        }
    }

    if block.is_none() {
        return ApiResponse::BadNoInfo(StatusCode::SERVICE_UNAVAILABLE);
    }
    let block = block.unwrap();

    let mut password_hash = [0; 64];

    Argon2::new(
        HASHER_ALGORITHM,
        HASHER_VERSION,
        HASHER_PARAMETERS.clone().unwrap(),
    )
    .hash_password_into_with_memory(
        &json.password_hash,
        &password_salt,
        &mut password_hash,
        **block,
    );

    let new_user: NewUser = NewUser {
        username: &json.username,
        password: &password_hash,
        salt: &password_salt,
        email: &json.email,
    };
    insert_into(users)
        .values(new_user)
        .execute(&mut db_connection)
        .unwrap();

    ApiResponse::Good(http::StatusCode::OK, Json(()))
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(usr_id): Path<Uuid>,
) -> ApiResponse<PublicUser> {
    let mut conn = get_conn_async(&state.pool).await;
    if let Ok(Some(user)) = users
        .select(PublicUser::as_select())
        .filter(id.eq(usr_id))
        .first(&mut conn)
        .optional()
    {
        return ApiResponse::Good(StatusCode::OK, Json(user));
    }
    ApiResponse::BadNoInfo(StatusCode::NOT_FOUND)
}

pub fn users_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(USERS_ROUTE, post(sign_up))
        .route(USER_ID_ROUTE, get(get_user))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_auth,
        ))
        .with_state(app_state)
}
