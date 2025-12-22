use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth::check_auth;
use crate::models;
use crate::models::*;
use crate::schema::licenses;
use crate::schema::objects;
use crate::schema::tags;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use axum::Extension;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get};
use diesel::insert_into;
use diesel::prelude::*;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use diesel_async::scoped_futures::ScopedFutureExt;

use futures_util::TryStreamExt;
use serde::Deserialize;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

const OBJECT_INFO_ROUTE: &str = "/{object_type}/{uuid}";
const OBJECT_DOWNLOAD_ROUTE: &str = constcat::concat!(OBJECT_INFO_ROUTE, "/epck");
const OBJECT_IMAGE_ROUTE: &str = constcat::concat!(OBJECT_INFO_ROUTE, "/image");

#[derive(Deserialize)]
pub struct ObjectUpload {
    name: String,
    description: String,
    tags: Vec<String>,
    flags: Vec<bool>,
    publicity: i16,
    license: String,
    encryption_key: Vec<u8>,
    encryption_iv: Vec<u8>,
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
                        StatusCode::FORBIDDEN,
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
                    .first::<i32>(&mut conn)
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

                let license: i32;
                if let Some(license_number) = licenses::table
                    .select(licenses::license)
                    .filter(licenses::text.eq(&json.license))
                    .first::<i32>(&mut conn)
                    .await
                    .optional()?
                {
                    license = license_number;
                } else {
                    license = insert_into(licenses::table)
                        .values(licenses::text.eq(&json.license))
                        .returning(licenses::license)
                        .get_result(&mut conn)
                        .await?;
                }

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
                    encryption_key: json.encryption_key,
                    encryption_iv: json.encryption_iv,
                    license: license,
                };

                diesel::insert_into(objects::table)
                    .values(object)
                    .execute(&mut conn)
                    .await?;

                for tag in json.tags {
                    insert_into(tags::table)
                        .values((tags::tag.eq(tag), tags::object.eq(object_id)))
                        .execute(&mut conn)
                        .await?;
                }
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
    let enum_str: &'static str = match object_type {
        ObjectType::World => "worlds",
        ObjectType::Avatar => "avatars",
    };

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
    Extension(user_id): Extension<Uuid>,
    body: Body,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;

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
                StatusCode::FORBIDDEN,
                Json(ErrorInfo {
                    error_code: ErrorCode::InsufficientPermissions,
                    error_message: Some(
                        "You do not have permission to edit this object.".to_owned(),
                    ),
                }),
            ));
        }

        let stream = body.into_data_stream();

        let enum_str: &'static str = match object_type {
            ObjectType::World => "worlds",
            ObjectType::Avatar => "avatars",
        };

        diesel::update(objects::table)
            .filter(objects::id.eq(&object_id))
            .filter(objects::object_type.eq(object_type as i16))
            .set((
                objects::verified.eq(false),
                objects::updated_at.eq(SystemTime::now()),
            ))
            .execute(&mut conn)
            .await?;

        upload_object_stream(
            &state.s3_client,
            enum_str,
            &object_id.to_string(),
            &mut tokio_util::io::StreamReader::new(stream.map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "no error handling here")
            })),
        )
        .await?;
    } else {
        return Err(ApiError::WithCode(StatusCode::NOT_FOUND));
    }

    Ok(())
}

pub async fn get_object_image(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
) -> Result<Body, ApiError> {
    let enum_str: &'static str = match object_type {
        ObjectType::World => "worlds",
        ObjectType::Avatar => "avatars",
    };

    let object = state
        .s3_client
        .get_object()
        .bucket(enum_str.to_owned() + "-images")
        .key(object_id.to_string())
        .send()
        .await?;
    let x = object.body.into_async_read();
    Ok(Body::from_stream(tokio_util::io::ReaderStream::new(x)))
}

pub async fn change_object_image(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
    Extension(user_id): Extension<Uuid>,
    body: Body,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;

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
                StatusCode::FORBIDDEN,
                Json(ErrorInfo {
                    error_code: ErrorCode::InsufficientPermissions,
                    error_message: Some(
                        "You do not have permission to edit this object.".to_owned(),
                    ),
                }),
            ));
        }

        let stream = body.into_data_stream();

        let enum_str: &'static str = match object_type {
            ObjectType::World => "worlds",
            ObjectType::Avatar => "avatars",
        };

        diesel::update(objects::table)
            .filter(objects::id.eq(&object_id))
            .filter(objects::object_type.eq(object_type as i16))
            .set((
                objects::verified.eq(false),
                objects::updated_at.eq(SystemTime::now()),
            ))
            .execute(&mut conn)
            .await?;

        upload_object_stream(
            &state.s3_client,
            &(enum_str.to_owned() + "-images"),
            &object_id.to_string(),
            &mut tokio_util::io::StreamReader::new(stream.map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "no error handling here")
            })),
        )
        .await?;
    } else {
        return Err(ApiError::WithCode(StatusCode::NOT_FOUND));
    }

    Ok(())
}

async fn upload_object_stream<S: AsyncRead + Unpin + Send>(
    client: &Client,
    bucket: &str,
    key: &str,
    stream: &mut S,
) -> Result<(), ApiError> {
    // 1MB
    const CHUNK_SIZE: usize = 1024 * 1024;

    let multipart_upload = client
        .create_multipart_upload()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    let upload_id = multipart_upload
        .upload_id
        .ok_or(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut parts: Vec<aws_sdk_s3::types::CompletedPart> = vec![];

    loop {
        let mut chunk = vec![0u8; CHUNK_SIZE];
        let read_size: usize = stream.read(&mut chunk).await?;
        chunk.resize(read_size, 0);

        if chunk.len() == 0 {
            break;
        }

        let part_number = parts.len() as i32 + 1;

        let part = client
            .upload_part()
            .bucket(bucket)
            .key(key)
            .part_number(part_number)
            .upload_id(&upload_id)
            .body(ByteStream::from(chunk))
            .send()
            .await?;
        let part = aws_sdk_s3::types::CompletedPart::builder()
            .e_tag(
                part.e_tag()
                    .ok_or(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?,
            )
            .part_number(part_number)
            .build();
        parts.push(part);
    }

    let completed_multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
        .set_parts(Some(parts))
        .build();
    client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(&upload_id)
        .multipart_upload(completed_multipart_upload)
        .send()
        .await?;

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
