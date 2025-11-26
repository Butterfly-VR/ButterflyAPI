use crate::ApiResponse;
use crate::AppState;
use crate::ROUTE_ORIGIN;
use crate::auth;
use crate::auth::check_auth;
use crate::models::*;
use crate::schema::users::dsl::*;
use crate::users;
use argon2::PasswordHash;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use password_hash::SaltString;
use password_hash::rand_core;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tokio::time::Instant;
use tokio::time::sleep;
use tracing::trace;
use tracing::warn;
use uuid::Uuid;

const TOKEN_ROUTE: &str = constcat::concat!(ROUTE_ORIGIN, "/token");
const TOKEN_VALIDATE_ROUTE: &str = constcat::concat!(ROUTE_ORIGIN, "/token/validate");
const TOKEN_USER_ROUTE: &str = constcat::concat!(ROUTE_ORIGIN, "/token/user");

#[derive(Deserialize)]
struct SignInRequest {
    email: String,
    password_hash: Vec<u8>,
    allow_renew: bool,
}

#[derive(Serialize)]
struct SignInResponse {
    token: Vec<u8>,
    token_expiry: SystemTime,
    renewable: bool,
}

pub async fn sign_in(
    State(state): State<Arc<AppState>>,
    Json(json): Json<SignInRequest>,
) -> ApiResponse<SignInResponse> {
}

pub async fn renew(State(state): State<Arc<AppState>>) -> ApiResponse<SignInResponse> {}

pub async fn verify() -> StatusCode {
    StatusCode::OK
}

pub async fn get_user(State(state): State<Arc<AppState>>) -> ApiResponse<SignInResponse> {}

pub fn tokens_router(app_state: Arc<AppState>) -> Router {
    let auth_router = Router::new()
        .route(TOKEN_ROUTE, get(renew))
        .route(TOKEN_VALIDATE_ROUTE, get(verify))
        .route(TOKEN_USER_ROUTE, get(get_user))
        .layer(middleware::from_fn_with_state(app_state, check_auth))
        .with_state(app_state);
    Router::new()
        .route(TOKEN_ROUTE, post(sign_in))
        .with_state(app_state)
        .merge(auth_router)
}
