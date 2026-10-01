use crate::ApiError;
use crate::AppState;
use crate::auth;
use crate::models::ObjectType;
use crate::models::PermissionsLevel;
use crate::models::User;
use crate::permission_checker;
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

pub fn moderation_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_auth,
        ))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            |state, req, next| {
                permission_checker::check_permissions(
                    vec![PermissionsLevel::Moderator, PermissionsLevel::Admin],
                    state,
                    req,
                    next,
                )
            },
        ))
        .with_state(app_state)
}
