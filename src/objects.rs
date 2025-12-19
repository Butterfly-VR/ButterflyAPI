use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth::check_auth;
use crate::hash::hash_password;
use crate::models;
use crate::models::*;
use crate::schema::objects;
use axum::Extension;
use axum::body::Body;
use axum::body::BodyDataStream;
use axum::debug_handler;
use axum::extract::Path;
use axum::extract::Request;
use axum::extract::State;
use axum::http;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get, routing::post};
use bytes::Bytes;
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use diesel_async::scoped_futures::ScopedFutureExt;
use futures_util::{Stream, TryStreamExt};
use rand_core::TryRngCore;
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

const OBJECT_INFO_ROUTE: &str = "/{object_type}/{uuid}";
const OBJECT_DOWNLOAD_ROUTE: &str = constcat::concat!(OBJECT_INFO_ROUTE, "/download");
const OBJECT_IMAGE_ROUTE: &str = constcat::concat!(OBJECT_INFO_ROUTE, "/image");

#[derive(Deserialize)]
pub struct ObjectUpload {
    name: String,
    description: String,
    tags: Vec<String>,
    publicity: i16,
    license: i32,
    custom_license: Option<String>,
}

pub async fn create_or_update_object(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
    Extension(user_id): Extension<Uuid>,
    Json(json): Json<ObjectUpload>,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;

    conn.transaction(|mut conn| {
        async move {
            if let Some(object) = objects::table
                .select(Object::as_select())
                .filter(objects::id.eq(&object_id))
                .filter(objects::object_type.eq(object_type as i16))
                .first(&mut conn)
                .await
                .optional()?
            {
                if object.creator != user_id {
                    return Err(ApiError::WithResponse(
                        StatusCode::BAD_REQUEST,
                        Json(ErrorInfo {
                            error_code: ErrorCode::InsufficientPermissions,
                            error_message: Some(
                                "You do not have permission to edit this object.".to_owned(),
                            ),
                        }),
                    ));
                }
                let mut new_object: Object = object.clone();
                new_object.name = json.name;
                new_object.description = json.description;
                new_object.publicity = json.publicity;

                if json.license != 3{
                    new_object.license = json.license
                }
                else{
                    if let Some(license_number) =
                }

                diesel::update(&object).set(new_object);
            }

            Ok(())
        }
        .scope_boxed()
    })
    .await
}

pub async fn get_object_info(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
) -> Result<Json<models::Object>, ApiError> {
    Err(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))
}

pub async fn get_object_file(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
) -> Result<Body, ApiError> {
    // todo: s3 object streamer with range requests?
    Err(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))
}

pub async fn change_object_file(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
    request: Request,
) -> Result<(), ApiError> {
    Ok(())
}

pub async fn get_object_image(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
) -> Result<Body, ApiError> {
    // todo: s3 object streamer with range requests?
    Err(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))
}

pub async fn change_object_image(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
    request: Request,
) -> Result<(), ApiError> {
    Ok(())
}

async fn stream_to_s3<S, E>(bucket: String, object: Uuid, stream: S) -> Result<(), ApiError>
where
    S: Stream<Item = Result<Bytes, E>>,
{
    Ok(())
}

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
