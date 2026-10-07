use crate::ApiError;
use crate::AppState;
use crate::ErrorCode;
use crate::ErrorInfo;
use crate::models::Object;
use crate::models::ObjectType;
use crate::schema::object_download_tokens;
use crate::schema::objects;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Json, Router, routing::get};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;
use sha2::Digest;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ObjectInfo {
    pub id: Uuid,
    pub name: String,
    pub verified: bool,
    pub creator: Uuid,
    pub object_type: i16,
    pub encryption_key: Vec<u8>,
    pub encryption_iv: Vec<u8>,
    pub server_variant_hash: Vec<u8>,
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

    let enum_str: &'static str = ObjectType::try_from(object.object_type)
        .map_err(|_| ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR))?
        .into();
    let enum_str = enum_str.to_owned() + "-server";

    let s3_object = state
        .s3_client
        .get_object()
        .bucket(enum_str)
        .key(object.id.to_string())
        .send()
        .await?;
    let mut x = s3_object.body.into_async_read();

    let mut buf = vec![0; 1024 * 1024];
    let mut hasher = sha2::Sha256::new();
    loop {
        let length = x.read(&mut buf).await?;
        if length == 0 {
            break;
        }
        hasher.update(&buf[..length]);
    }

    let object_info = ObjectInfo {
        id: object.id,
        name: object.name,
        verified: object.verified,
        creator: object.creator,
        object_type: object.object_type,
        encryption_key: object.encryption_key,
        encryption_iv: object.encryption_iv,
        server_variant_hash: hasher.finalize().to_vec(),
    };

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
    Ok(Body::from_stream(tokio_util::io::ReaderStream::new(x)))
}

pub fn object_download_token_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/token_details/{token}", get(get_object_info_from_token))
        .route("/spend_token/{token}", get(get_object_from_token))
        .with_state(app_state)
}
