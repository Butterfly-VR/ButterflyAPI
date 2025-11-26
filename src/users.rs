use crate::ApiResponse;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth;
use crate::hash::hash_password;
use crate::models::*;
use crate::schema::users::dsl::*;
use axum::extract::Path;
use axum::extract::State;
use axum::http;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use rand_core::TryRngCore;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

const USERS_ROUTE: &str = "/user";
const USER_ID_ROUTE: &str = constcat::concat!(USERS_ROUTE, "/{usr_id}");

#[derive(Deserialize)]
struct SignUpRequest {
    pub username: String,
    pub password_hash: Vec<u8>,
    pub email: String,
}

pub async fn sign_up(
    state: State<Arc<AppState>>,
    Json(json): Json<SignUpRequest>,
) -> ApiResponse<()> {
    let conn = state.pool.get().await;
    if conn.is_err() {
        return ApiResponse::BadNoInfo(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut conn = conn.unwrap();

    if users
        .count()
        .filter(username.eq(&json.username))
        .or_filter(email.eq(&json.email))
        .get_result(&mut conn)
        .await
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

    if let Some(password_hash) = hash_password(
        state.clone(),
        json.password_hash.try_into().unwrap(),
        password_salt.clone(),
    )
    .await
    .ok()
    {
        let new_user: NewUser = NewUser {
            username: &json.username,
            password: &password_hash,
            salt: &password_salt,
            email: &json.email,
        };
        if insert_into(users)
            .values(new_user)
            .execute(&mut conn)
            .await
            .is_ok()
        {
            return ApiResponse::Good(http::StatusCode::OK, Json(()));
        }
    } else {
        // most likely cause is that we didnt have a block available
        return ApiResponse::BadNoInfo(http::StatusCode::SERVICE_UNAVAILABLE);
    }
    return ApiResponse::BadNoInfo(http::StatusCode::INTERNAL_SERVER_ERROR);
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(usr_id): Path<Uuid>,
) -> ApiResponse<PublicUser> {
    let conn = state.pool.get().await;
    if conn.is_err() {
        return ApiResponse::BadNoInfo(StatusCode::SERVICE_UNAVAILABLE);
    }
    let mut conn = conn.unwrap();

    if let Ok(Some(user)) = users
        .select(PublicUser::as_select())
        .filter(id.eq(usr_id))
        .first(&mut conn)
        .await
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
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_auth,
        ))
        .with_state(app_state)
}
