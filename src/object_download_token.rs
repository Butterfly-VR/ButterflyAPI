use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::models::Object;
use crate::models::ObjectType;
use crate::objects::upload_object_stream;
use crate::schema::object_download_tokens;
use crate::schema::objects;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router, routing::get};
use diesel::prelude::*;
use diesel::update;
use diesel_async::RunQueryDsl;
use futures_util::TryStreamExt;
use serde::Serialize;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

const TOKEN_STAGE1_PATH: &str = "/token_details/{token}";
const TOKEN_STAGE2_PATH: &str = "/spend_token/{token}";
const TOKEN_STAGE3_PATH: &str = "/upload_server_variant/{token}";

#[derive(Serialize)]
pub struct ObjectInfo {
    pub id: Uuid,
    pub name: String,
    pub verified: bool,
    pub creator: Uuid,
    pub object_type: i16,
    pub encryption_key: Vec<u8>,
    pub encryption_iv: Vec<u8>,
}

pub async fn get_object_info_from_token(
    State(state): State<Arc<AppState>>,
    Path(token): Path<Uuid>,
) -> Result<Json<ObjectInfo>, ApiError> {
    let mut conn = state.pool.get().await?;

    let Some(object) = object_download_tokens::table
        .inner_join(objects::table)
        .select(Object::as_select())
        .filter(object_download_tokens::token.eq(token))
        .filter(object_download_tokens::expiry.gt(SystemTime::now()))
        .filter(object_download_tokens::current_step.eq(0))
        .first(&mut conn)
        .await
        .optional()?
    else {
        return Err(ApiError::WithResponse(
            StatusCode::NOT_FOUND,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: None,
            }),
        ));
    };

    let object_info = ObjectInfo {
        id: object.id,
        name: object.name,
        verified: object.verified,
        creator: object.creator,
        object_type: object.object_type,
        encryption_key: object.encryption_key,
        encryption_iv: object.encryption_iv,
    };

    update(object_download_tokens::table)
        .set(object_download_tokens::current_step.eq(1))
        .filter(object_download_tokens::token.eq(token))
        .execute(&mut conn)
        .await?;

    return Ok(Json(object_info));
}

pub async fn get_object_from_token(
    State(state): State<Arc<AppState>>,
    Path(token): Path<Uuid>,
) -> Result<Body, ApiError> {
    let mut conn = state.pool.get().await?;

    let Some(object) = object_download_tokens::table
        .inner_join(objects::table)
        .select(Object::as_select())
        .filter(object_download_tokens::token.eq(token))
        .filter(object_download_tokens::expiry.gt(SystemTime::now()))
        .filter(object_download_tokens::current_step.eq(1))
        .first(&mut conn)
        .await
        .optional()?
    else {
        return Err(ApiError::WithResponse(
            StatusCode::NOT_FOUND,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: None,
            }),
        ));
    };

    let object_type = ObjectType::try_from(object.object_type)
        .map_err(|_| ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?;

    let enum_str: &'static str = object_type.into();
    let object = state
        .s3_client
        .get_object()
        .bucket(enum_str.to_owned())
        .key(object.id.to_string())
        .send()
        .await?;
    let x = object.body.into_async_read();

    update(object_download_tokens::table)
        .filter(object_download_tokens::token.eq(token))
        .set(object_download_tokens::current_step.eq(2))
        .execute(&mut conn)
        .await?;

    Ok(Body::from_stream(tokio_util::io::ReaderStream::new(x)))
}

pub async fn change_object_server_variant_file(
    state: State<Arc<AppState>>,
    Path(token): Path<Uuid>,
    body: Body,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;

    let Some(object) = object_download_tokens::table
        .inner_join(objects::table)
        .select(Object::as_select())
        .filter(object_download_tokens::token.eq(token))
        .filter(object_download_tokens::expiry.gt(SystemTime::now()))
        .filter(object_download_tokens::current_step.eq(2))
        .first(&mut conn)
        .await
        .optional()?
    else {
        return Err(ApiError::WithResponse(
            StatusCode::NOT_FOUND,
            Json(ErrorInfo {
                error_code: ErrorCode::DosentExist,
                error_message: None,
            }),
        ));
    };

    let object: Object = object;

    state.server_variant_cache.invalidate(&object.id).await;

    let stream = body.into_data_stream();

    let enum_str: &'static str = match object
        .object_type
        .try_into()
        .map_err(|_| ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?
    {
        ObjectType::World => "worlds",
        ObjectType::Avatar => "avatars",
    };
    let enum_str = enum_str.to_owned() + "-server";

    upload_object_stream(
        &state.s3_client,
        &enum_str,
        &object.id.to_string(),
        &mut tokio_util::io::StreamReader::new(stream.map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "no error handling here")
        })),
        object.object_size as usize / 1024,
    )
    .await?;

    update(object_download_tokens::table)
        .filter(object_download_tokens::token.eq(token))
        .set(object_download_tokens::current_step.eq(3))
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub fn object_download_token_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(TOKEN_STAGE1_PATH, get(get_object_info_from_token))
        .route(TOKEN_STAGE2_PATH, get(get_object_from_token))
        .route(TOKEN_STAGE3_PATH, post(change_object_server_variant_file))
        .with_state(app_state)
}
