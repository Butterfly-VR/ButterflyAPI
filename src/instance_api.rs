use crate::ApiError;
use crate::AppState;
use crate::auth;
use crate::models::*;
use crate::schema::instances;
use crate::schema::objects;
use crate::schema::tags;
use crate::schema::users;
use axum::Extension;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::{Json, Router, routing::get};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use uuid::Uuid;

const INSTANCE_API_ROUTE: &str = "/internal";
const INSTANCE_CLOSE_ROUTE: &str = constcat::concat!(INSTANCE_API_ROUTE, "/close_instance");
const INSTANCE_USER_ROUTE: &str = constcat::concat!(INSTANCE_API_ROUTE, "/user/{user_id}");
const INSTANCE_OBJECT_ROUTE: &str = constcat::concat!(INSTANCE_API_ROUTE, "/object/{object_id}");

pub async fn verify_instance_token() -> StatusCode {
    StatusCode::OK
}

pub async fn close_instance(
    State(state): State<Arc<AppState>>,
    Extension(id): Extension<Uuid>,
) -> Result<(), ApiError> {
    let mut conn = state.pool.get().await?;

    if diesel::delete(instances::table)
        .filter(instances::id.eq(id))
        .execute(&mut conn)
        .await?
        != 0
    {
        Ok(())
    } else {
        // since this endpoint requires a valid token, this could only trigger
        // if the instance was deleted while this was running
        Err(ApiError::WithCode(StatusCode::NOT_FOUND))
    }
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<PublicUserInfo>, ApiError> {
    let mut conn = state.pool.get().await?;

    let user = users::table
        .select(PublicUserInfo::as_select())
        .filter(users::id.eq(user_id))
        .first(&mut conn)
        .await
        .optional()?;

    let Some(user) = user else {
        return Err(ApiError::WithCode(StatusCode::NOT_FOUND));
    };

    Ok(Json(user))
}

pub async fn get_object(
    State(state): State<Arc<AppState>>,
    Path(object_id): Path<Uuid>,
) -> Result<Json<crate::objects::ObjectInfo>, ApiError> {
    let mut conn = state.pool.get().await?;

    let object = objects::table
        .select(Object::as_select())
        .filter(objects::id.eq(object_id))
        .first(&mut conn)
        .await
        .optional()?;

    let Some(object) = object else {
        return Err(ApiError::WithCode(StatusCode::NOT_FOUND));
    };

    let tags = tags::table
        .select(tags::tag)
        .filter(tags::object.eq(object.id))
        .load(&mut conn)
        .await?;
    Ok(Json(crate::objects::ObjectInfo {
        id: object.id,
        name: object.name,
        description: object.description,
        flags: object
            .flags
            .iter()
            .map(|x| if let Some(x) = x { *x } else { false })
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
}

pub fn instance_api_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(INSTANCE_API_ROUTE, get(verify_instance_token))
        .route(INSTANCE_CLOSE_ROUTE, get(close_instance))
        .route(INSTANCE_USER_ROUTE, get(get_user))
        .route(INSTANCE_OBJECT_ROUTE, get(get_object))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::check_instance_auth,
        ))
        .with_state(app_state)
}
