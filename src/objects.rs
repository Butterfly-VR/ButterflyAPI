use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth::check_auth;
use crate::hash::hash_password;
use crate::models;
use crate::models::*;
use crate::schema::licenses;
use crate::schema::objects;
use crate::schema::tags;
use aws_sdk_s3::primitives::ByteStream;
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
use std::io::Write;
use std::sync::Arc;
use std::time::SystemTime;
use tempfile::SpooledTempFile;
use tokio::io::AsyncBufRead;
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
    flags: Vec<bool>,
    publicity: i16,
    license: String,
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
                // update existing object
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

                new_object.updated_at = SystemTime::now();

                if let Some(license_number) = licenses::table
                    .select(licenses::license)
                    .filter(licenses::text.eq(&json.license))
                    .get_result::<i32>(&mut conn)
                    .await
                    .optional()?
                {
                    new_object.license = license_number;
                } else {
                    new_object.license = insert_into(licenses::table)
                        .values(licenses::text.eq(&json.license))
                        .returning(licenses::license)
                        .get_result(&mut conn)
                        .await?;
                }

                // delete all previous tags before readding
                // would probably be faster to get existing tags and only delete / insert the diff
                diesel::delete(tags::table)
                    .filter(tags::object.eq(object_id))
                    .execute(&mut conn)
                    .await?;

                for tag in json.tags {
                    insert_into(tags::table)
                        .values((tags::tag.eq(tag), tags::object.eq(object_id)))
                        .execute(&mut conn)
                        .await?;
                }

                diesel::update(&object)
                    .set(new_object)
                    .execute(&mut conn)
                    .await?;
            } else {
                // create new object
                let object: Object = Object {
                    id: object_id,
                    name: json.name,
                    description: json.description,
                    flags: json.flags.into_iter().map(|x| Some(x)).collect(),
                    updated_at: SystemTime::now(),
                    created_at: SystemTime::now(),
                    verified: false,
                    object_size: 0,
                    image_size: 0,
                    creator: user_id,
                    object_type: object_type as i16,
                    publicity: json.publicity,
                    license: insert_into(licenses::table)
                        .values(licenses::text.eq(&json.license))
                        .returning(licenses::license)
                        .get_result(&mut conn)
                        .await?,
                };

                for tag in json.tags {
                    insert_into(tags::table)
                        .values((tags::tag.eq(tag), tags::object.eq(object_id)))
                        .execute(&mut conn)
                        .await?;
                }

                diesel::insert_into(objects::table)
                    .values(object)
                    .execute(&mut conn)
                    .await?;
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
    let mut conn = state.pool.get().await?;

    objects::table
        .select(Object::as_select())
        .filter(objects::id.eq(&object_id))
        .filter(objects::object_type.eq(object_type as i16))
        .first(&mut conn)
        .await
        .optional()?
        .map(|x| Json(x))
        .ok_or(ApiError::WithCode(StatusCode::NOT_FOUND))
}

pub async fn get_object_file(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
) -> Result<Body, ApiError> {
    let enum_str: &'static str = object_type.into();
    let object = state
        .s3_client
        .get_object()
        .bucket(enum_str.to_owned())
        .key(object_id.to_string())
        .send()
        .await?;
    let x = object.body.into_async_read();
    Ok(Body::from_stream(tokio_util::io::ReaderStream::new(x)))
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
    let enum_str: &'static str = object_type.into();
    let object = state
        .s3_client
        .get_object()
        .bucket(enum_str.to_owned() + "_images")
        .key(object_id.to_string())
        .send()
        .await?;
    let x = object.body.into_async_read();
    Ok(Body::from_stream(tokio_util::io::ReaderStream::new(x)))
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
