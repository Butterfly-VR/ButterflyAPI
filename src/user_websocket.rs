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

#[derive(Serialize)]
enum WSMessage {
    UserChatMessage(Uuid, String),
    ServerChatMessage(String),
    Notification {
        catagory: usize,
        priority: usize,
        icon: Uuid,
        message: String,
    },
}

pub async fn start_user_websocket(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ApiError> {
    Ok(ws.on_upgrade(|x| handle_user_websocket(state, x)))
}

pub async fn handle_user_websocket(state: Arc<AppState>, mut ws: WebSocket) {
    // poll db for relevant changes and pass to user
    // poll slower with less activity using exponential backoff
    const DEFAULT_BACKOFF: Duration = Duration::from_millis(100);
    const MAX_BACKOFF: Duration = Duration::from_millis(10000);
    let mut backoff = DEFAULT_BACKOFF;

    let tick = tokio::time::interval(backoff);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut conn = state.pool.get().await.unwrap();

    while let Some(msg) = ws.recv().await {
        // todo: finish this when we actually have messages to poll for
        tokio::time::sleep(backoff).await;
        backoff = backoff.saturating_mul(2).min(MAX_BACKOFF);
    }
}
