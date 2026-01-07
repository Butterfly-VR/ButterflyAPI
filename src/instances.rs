use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth;
use crate::email::EmailType;
use crate::email::check_email;
use crate::email::send_email;
use crate::hash::hash_password;
use crate::models::*;
use crate::schema::unverified_users;
use crate::schema::users;
use axum::Extension;
use axum::extract::Path;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::http;
use axum::http::StatusCode;
use axum::middleware;
use axum::response::IntoResponse;
use axum::{Json, Router, routing::get, routing::post};
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use futures_util::StreamExt;
use futures_util::stream::SplitSink;
use rand_core::TryRngCore;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

const INSTANCES_ROUTE: &str = "/instances";
const INSTANCE_ID_ROUTE: &str = constcat::concat!(INSTANCES_ROUTE, "/{id}");
const INSTANCE_JOIN_ROUTE: &str = constcat::concat!(INSTANCE_ID_ROUTE, "/join");

pub fn instances_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(INSTANCES_ROUTE, post(create_instance))
        .route(INSTANCE_ID_ROUTE, get(get_instance))
        .route(INSTANCE_JOIN_ROUTE, get(join_instance))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_auth,
        ))
        .with_state(app_state)
}
