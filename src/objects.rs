use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth::check_auth;
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
use tracing::info;
use uuid::Uuid;

const OBJECT_INFO_ROUTE: &str = "/{object_type}/{uuid}";
const OBJECT_DOWNLOAD_ROUTE: &str = constcat::concat!(OBJECT_INFO_ROUTE, "/download");
const OBJECT_IMAGE_ROUTE: &str = constcat::concat!(OBJECT_INFO_ROUTE, "/image");

pub fn objects_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            OBJECT_INFO_ROUTE,
            get(get_object_info).post(create_or_update_object),
        )
        .route(
            OBJECT_DOWNLOAD_ROUTE,
            get(get_object_file).post(change_object_file),
        )
        .route(
            OBJECT_IMAGE_ROUTE,
            get(get_object_image).post(change_object_image),
        )
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            check_auth,
        ))
        .with_state(app_state.clone())
}
