use crate::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use schema::tokens::dsl::*;
use std::{sync::Arc, time::SystemTime};
use uuid::Uuid;

use crate::schema;

pub async fn check_auth(
    state: State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let header_token = req
        .headers()
        .get("token")
        .and_then(|x| hex::decode(x).ok())
        .unwrap_or_default();
    if let Ok(Some(user_id)) = tokens
        .select(user)
        .filter(token.eq(header_token))
        .filter(expiry.gt(SystemTime::now()))
        .first::<Uuid>(&mut conn)
        .await
        .optional()
    {
        req.extensions_mut().insert(user_id);
        return Ok(next.run(req).await);
    }
    Err(StatusCode::UNAUTHORIZED)
}
