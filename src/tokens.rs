use crate::AppState;
use crate::ROUTE_ORIGIN;
use crate::auth;
use crate::models::*;
use crate::schema::users::dsl::*;
use crate::users;
use argon2::PasswordHash;
use axum::extract::Path;
use axum::extract::State;
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

const USERS_ROUTE: &str = constcat::concat!(ROUTE_ORIGIN, "/user");
const USER_ID_ROUTE: &str = constcat::concat!(USERS_ROUTE, "/{usr_id}");

pub fn tokens_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            USERS_ROUTE,
            get(find_user)
                .layer(middleware::from_fn(auth::check_auth))
                .post(sign_up),
        )
        .route(constcat::concat!(USERS_ROUTE, "/sign_in"), post(sign_in))
        .route(USER_ID_ROUTE, get(get_user))
        .with_state(app_state)
}
