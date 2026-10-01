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

pub fn object_download_token_router(app_state: Arc<AppState>) -> Router {
    Router::new().with_state(app_state)
}
