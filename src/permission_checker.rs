use crate::models::PermissionsLevel;
use crate::schema::users;
use crate::{ApiError, AppState};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::sync::Arc;
use tracing::{debug, trace};
use uuid::Uuid;

pub async fn check_permissions(
    required_levels: Vec<PermissionsLevel>,
    state: State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let mut conn = state.pool.get().await?;
    let Some(user) = req.extensions().get::<Uuid>() else {
        return Err(ApiError::WithCode(StatusCode::INTERNAL_SERVER_ERROR));
    };

    if required_levels.contains(
        &users::table
            .select(users::permissions_level)
            .filter(users::id.eq(user))
            .first::<i16>(&mut conn)
            .await?
            .into(),
    ) {
        trace!(
            "authenticated user {:?} for access to {:?} with one of permissions levels: {:?}",
            user,
            req.uri(),
            required_levels
        );
        return Ok(next.run(req).await);
    }
    debug!("forbidden request to {:?}", req.uri());
    Err(ApiError::WithCode(StatusCode::FORBIDDEN))
}
