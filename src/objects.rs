use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::auth::check_auth;
use crate::models;
use crate::models::ObjectPublicity;
use crate::models::{Object, ObjectType};
use crate::schema::licenses;
use crate::schema::objects;
use crate::schema::tags;
use crate::schema::users;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use axum::Extension;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get};
use diesel::dsl::sql;
use diesel::insert_into;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::AsyncConnection;
use diesel_async::RunQueryDsl;
use futures_util::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

const OBJECT_INFO_ROUTE: &str = "/{object_type}/{uuid}";
const OBJECT_DOWNLOAD_ROUTE: &str = constcat::concat!(OBJECT_INFO_ROUTE, "/epck");
const OBJECT_IMAGE_ROUTE: &str = constcat::concat!(OBJECT_INFO_ROUTE, "/image");
const MAX_TOTAL_UPLOADED_KB: usize = 1024 * 100;
const MAX_OBJECTS_PER_USER: i64 = 100;

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

    if json.name.len() < 6
        || json.name.len() > 32
        || json.description.len() > 4096
        || json.license.len() > 1024 * 1024 * 10
    {
        return Err(ApiError::WithResponse(
            StatusCode::BAD_REQUEST,
            Json(ErrorInfo {
                error_code: ErrorCode::BadRequestLength,
                error_message: Some(String::from(
                    "Name or description was wrong length. This shouldnt happen",
                )),
            }),
        ));
    }

    for tag in &json.tags {
        if tag.len() < 3 || tag.len() > 32 {
            return Err(ApiError::WithResponse(
                StatusCode::BAD_REQUEST,
                Json(ErrorInfo {
                    error_code: ErrorCode::BadRequestLength,
                    error_message: Some(format!(
                        "Tag {tag} was wrong length. This shouldnt happen",
                    )),
                }),
            ));
        }
    }

    conn.transaction(async |mut conn| {
        {
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
                new_object.flags = json.flags.into_iter().map(Some).collect();
                new_object.encryption_key = json.encryption_key;
                new_object.encryption_iv = json.encryption_iv;

                new_object.updated_at = SystemTime::now();

                if let Some(license_number) = licenses::table
                    .select(licenses::id)
                    .filter(licenses::text.eq(&json.license))
                    .first::<Uuid>(&mut conn)
                    .await
                    .optional()?
                {
                    new_object.license = license_number;
                } else {
                    new_object.license = insert_into(licenses::table)
                        .values(licenses::text.eq(&json.license))
                        .returning(licenses::id)
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

                if objects::table
                    .select(diesel::dsl::count(objects::id))
                    .left_join(users::table.on(users::id.eq(objects::creator)))
                    .group_by(objects::creator)
                    .filter(users::id.eq(user_id))
                    .first::<i64>(&mut conn)
                    .await?
                    > MAX_OBJECTS_PER_USER
                {
                    return Err(ApiError::WithResponse(
                        StatusCode::BAD_REQUEST,
                        Json(ErrorInfo {
                            error_code: ErrorCode::InsufficientSpace,
                            error_message: Some(
                                "You have reached the maximum number of objects per user"
                                    .to_owned(),
                            ),
                        }),
                    ));
                }

                if objects::table
                    .count()
                    .filter(objects::name.eq(&json.name))
                    .first::<i64>(&mut conn)
                    .await?
                    != 0
                {
                    return Err(ApiError::WithResponse(
                        StatusCode::BAD_REQUEST,
                        Json(ErrorInfo {
                            error_code: ErrorCode::AlreadyExists,
                            error_message: Some(
                                "An object with that name already exists".to_owned(),
                            ),
                        }),
                    ));
                }

                let license = if let Some(license_number) = licenses::table
                    .select(licenses::id)
                    .filter(licenses::text.eq(&json.license))
                    .first::<Uuid>(&mut conn)
                    .await
                    .optional()?
                {
                    license_number
                } else {
                    insert_into(licenses::table)
                        .values((
                            licenses::text.eq(&json.license),
                            licenses::id.eq(uuid::Uuid::new_v4()),
                        ))
                        .returning(licenses::id)
                        .get_result(&mut conn)
                        .await?
                };

                let object: Object = Object {
                    id: object_id,
                    name: json.name,
                    description: json.description,
                    flags: json.flags.into_iter().map(Some).collect(),
                    updated_at: SystemTime::now(),
                    created_at: SystemTime::now(),
                    verified: false,
                    object_size: 0,
                    image_size: 0,
                    creator: user_id,
                    object_type: object_type as i16,
                    likes: 0,
                    dislikes: 0,
                    publicity: json.publicity,
                    encryption_key: json.encryption_key,
                    encryption_iv: json.encryption_iv,
                    license,
                    delete_at: None,
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
    })
    .await
}

#[derive(Serialize)]
pub struct ObjectInfo {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub flags: Vec<bool>,
    pub updated_at: u64,
    pub created_at: u64,
    pub object_size: i64,
    pub image_size: i64,
    pub creator: Uuid,
    pub object_type: i16,
    pub publicity: i16,
    pub license: Uuid,
    pub encryption_key: Vec<u8>,
    pub encryption_iv: Vec<u8>,
    pub tags: Vec<String>,
}

pub async fn get_object_info(
    state: State<Arc<AppState>>,
    Path((object_type, object_id)): Path<(models::ObjectType, Uuid)>,
) -> Result<Json<ObjectInfo>, ApiError> {
    let mut conn = state.pool.get().await?;

    if let Some(object) = objects::table
        .select(Object::as_select())
        .filter(objects::id.eq(&object_id))
        .filter(objects::object_type.eq(object_type as i16))
        .first::<Object>(&mut conn)
        .await
        .optional()?
    {
        let tags = tags::table
            .select(tags::tag)
            .filter(tags::object.eq(object.id))
            .load(&mut conn)
            .await?;
        Ok(Json(ObjectInfo {
            id: object.id,
            name: object.name,
            description: object.description,
            flags: object
                .flags
                .iter()
                .map(|x| x.unwrap_or(false))
                .collect::<Vec<bool>>(),
            updated_at: object
                .updated_at
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            created_at: object
                .created_at
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            object_size: object.object_size,
            image_size: object.image_size,
            creator: object.creator,
            object_type: object.object_type,
            publicity: object.publicity,
            license: object.license,
            encryption_iv: object.encryption_iv,
            encryption_key: object.encryption_key,
            tags,
        }))
    } else {
        Err(ApiError::WithResponse(
            StatusCode::NOT_FOUND,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: None,
            }),
        ))
    }
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

        let total_uploaded_objects_kb = objects::table
            .select(
                sql::<BigInt>("CAST(")
                    .bind(diesel::dsl::sum(objects::object_size))
                    .sql(" AS BIGINT)"),
            )
            .filter(objects::creator.eq(user_id))
            .filter(objects::publicity.ne(ObjectPublicity::Public as i16))
            .get_result::<i64>(&mut conn)
            .await
            .unwrap_or(0);
        let total_uploaded_images_kb = objects::table
            .select(
                sql::<BigInt>("CAST(")
                    .bind(diesel::dsl::sum(objects::image_size))
                    .sql(" AS BIGINT)"),
            )
            .filter(objects::creator.eq(user_id))
            .filter(objects::publicity.ne(ObjectPublicity::Public as i16))
            .get_result::<i64>(&mut conn)
            .await
            .unwrap_or(0);
        let total_uploaded_kb = total_uploaded_objects_kb + total_uploaded_images_kb;

        upload_object_stream(
            &state.s3_client,
            enum_str,
            &object_id.to_string(),
            &mut tokio_util::io::StreamReader::new(stream.map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "no error handling here")
            })),
            MAX_TOTAL_UPLOADED_KB - total_uploaded_kb as usize,
        )
        .await?;

        diesel::update(objects::table)
            .filter(objects::id.eq(&object_id))
            .filter(objects::object_type.eq(object_type as i16))
            .set(
                objects::object_size.eq(state
                    .s3_client
                    .head_object()
                    .bucket(enum_str)
                    .key(object_id.to_string())
                    .send()
                    .await?
                    .content_length()
                    .unwrap_or_default()),
            )
            .execute(&mut conn)
            .await?;
    } else {
        return Err(ApiError::WithResponse(
            StatusCode::NOT_FOUND,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: None,
            }),
        ));
    }

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

        let total_uploaded_objects_kb = objects::table
            .select(
                sql::<BigInt>("CAST(")
                    .bind(diesel::dsl::sum(objects::object_size))
                    .sql(" AS BIGINT)"),
            )
            .filter(objects::creator.eq(user_id))
            .filter(objects::publicity.ne(ObjectPublicity::Public as i16))
            .get_result::<i64>(&mut conn)
            .await
            .unwrap_or(0);
        let total_uploaded_images_kb = objects::table
            .select(
                sql::<BigInt>("CAST(")
                    .bind(diesel::dsl::sum(objects::image_size))
                    .sql(" AS BIGINT)"),
            )
            .filter(objects::creator.eq(user_id))
            .filter(objects::publicity.ne(ObjectPublicity::Public as i16))
            .get_result::<i64>(&mut conn)
            .await
            .unwrap_or(0);
        let total_uploaded_kb = total_uploaded_objects_kb + total_uploaded_images_kb;

        upload_object_stream(
            &state.s3_client,
            &(enum_str.to_owned() + "-images"),
            &object_id.to_string(),
            &mut tokio_util::io::StreamReader::new(stream.map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "no error handling here")
            })),
            MAX_TOTAL_UPLOADED_KB - total_uploaded_kb as usize,
        )
        .await?;

        diesel::update(objects::table)
            .filter(objects::id.eq(&object_id))
            .filter(objects::object_type.eq(object_type as i16))
            .set(
                objects::image_size.eq(state
                    .s3_client
                    .head_object()
                    .bucket(&(enum_str.to_owned() + "-images"))
                    .key(object_id.to_string())
                    .send()
                    .await?
                    .content_length()
                    .unwrap_or_default()),
            )
            .execute(&mut conn)
            .await?;
    } else {
        return Err(ApiError::WithResponse(
            StatusCode::NOT_FOUND,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: None,
            }),
        ));
    }

    Ok(())
}

async fn upload_object_stream<S: AsyncRead + Unpin + Send>(
    client: &Client,
    bucket: &str,
    key: &str,
    stream: &mut S,
    available_space_kb: usize,
) -> Result<(), ApiError> {
    // 10MB
    const CHUNK_SIZE: usize = 10 * 1024 * 1024;
    const MAX_PUT_SIZE: usize = CHUNK_SIZE * 2;

    // dont bother with multipart if smaller than MAX_PUT_SIZE
    let mut first_chunk = vec![0u8; MAX_PUT_SIZE];

    let mut total_read_size: usize = 0;
    loop {
        let read_size: usize = stream.read(&mut first_chunk[total_read_size..]).await?;
        if read_size == 0 {
            break;
        }
        total_read_size += read_size;

        if total_read_size == MAX_PUT_SIZE {
            break;
        }
    }
    first_chunk.truncate(total_read_size);

    if first_chunk.len() < MAX_PUT_SIZE {
        if first_chunk.len() > (available_space_kb * 1024) {
            return Err(ApiError::WithResponse(
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorInfo {
                    error_code: ErrorCode::InsufficientSpace,
                    error_message: Some("Not enough space".to_string()),
                }),
            ));
        }
        client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(ByteStream::from(first_chunk))
            .send()
            .await?;
        return Ok(());
    }

    let mut object_size: usize = first_chunk.len();

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

    for chunk in first_chunk.chunks_exact(CHUNK_SIZE) {
        object_size += CHUNK_SIZE;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let part_number = parts.len() as i32 + 1;
        let part = client
            .upload_part()
            .bucket(bucket)
            .key(key)
            .part_number(part_number)
            .upload_id(&upload_id)
            .body(ByteStream::from(chunk.to_owned()))
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

    let remainder = first_chunk.chunks_exact(CHUNK_SIZE).remainder();
    let remainder_len = remainder.len();
    let mut chunk = remainder.iter().copied().collect::<Vec<_>>();
    chunk.resize(CHUNK_SIZE, 0);
    let mut total_read_size = remainder_len;

    loop {
        loop {
            let read_size: usize = stream.read(&mut chunk[total_read_size..]).await?;
            if read_size == 0 {
                break;
            }
            total_read_size += read_size;
            if total_read_size == CHUNK_SIZE {
                break;
            }
        }
        chunk.resize(total_read_size, 0);
        object_size += total_read_size;

        if object_size > available_space_kb * 1024 {
            client
                .abort_multipart_upload()
                .bucket(bucket)
                .key(key)
                .upload_id(&upload_id)
                .send()
                .await?;
            return Err(ApiError::WithResponse(
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorInfo {
                    error_code: ErrorCode::InsufficientSpace,
                    error_message: Some("Not enough space".to_string()),
                }),
            ));
        }

        if chunk.is_empty() {
            break;
        }

        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
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

        chunk = vec![0u8; CHUNK_SIZE];
        total_read_size = 0;
    }

    let completed_multipart_upload = aws_sdk_s3::types::CompletedMultipartUpload::builder()
        .set_parts(Some(parts))
        .build();
    if client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(&upload_id)
        .multipart_upload(completed_multipart_upload)
        .send()
        .await
        .is_err()
    {
        client
            .abort_multipart_upload()
            .bucket(bucket)
            .key(key)
            .upload_id(&upload_id)
            .send()
            .await?;
        return Err(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR));
    }

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
        .with_state(app_state)
}
