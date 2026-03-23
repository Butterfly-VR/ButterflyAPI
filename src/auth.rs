use crate::{ApiError, AppState, schema::instances};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use schema::tokens::dsl::{expiry, token, tokens, user};
use std::{sync::Arc, time::SystemTime};
use uuid::Uuid;

use crate::schema;

pub async fn check_auth(
    state: State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let mut conn = state.pool.get().await?;

    let header_token = req
        .headers()
        .get("token")
        .and_then(|x| hex::decode(x).ok())
        .unwrap_or_default();
    if let Ok(Some(user_id)) = tokens
        .select(user)
        .filter(token.eq(&header_token))
        .filter(expiry.gt(SystemTime::now()))
        .first::<Uuid>(&mut conn)
        .await
        .optional()
    {
        req.extensions_mut().insert(user_id);
        return Ok(next.run(req).await);
    }
    if let Ok(Some(instance_id)) = instances::table
        .select(instances::id)
        .filter(instances::server_token.eq(&header_token))
        .first::<Uuid>(&mut conn)
        .await
        .optional()
    {
        req.extensions_mut().insert(instance_id);
        return Ok(next.run(req).await);
    }
    Err(ApiError::WithCode(StatusCode::UNAUTHORIZED))
}

pub async fn check_instance_auth(
    state: State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let mut conn = state.pool.get().await?;

    let header_token = req
        .headers()
        .get("token")
        .and_then(|x| hex::decode(x).ok())
        .unwrap_or_default();
    if let Ok(Some(user_id)) = instances::table
        .select(instances::id)
        .filter(instances::server_token.eq(header_token))
        .first::<Uuid>(&mut conn)
        .await
        .optional()
    {
        req.extensions_mut().insert(user_id);
        return Ok(next.run(req).await);
    }
    Err(ApiError::WithCode(StatusCode::UNAUTHORIZED))
}
