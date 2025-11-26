use crate::{AppState, get_conn_async};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use diesel::prelude::*;
use schema::tokens::dsl::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::schema;

pub async fn check_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut conn = get_conn_async(&state.pool).await;
    let header_token = req
        .headers()
        .get("token")
        .map(|x| x.as_bytes())
        .unwrap_or_default();
    if let Some(Some(user_id)) = tokens
        .select(user)
        .filter(token.eq(header_token))
        .first::<Uuid>(&mut conn)
        .optional()
        .ok()
    {
        req.extensions_mut().insert(user_id);
        return Ok(next.run(req).await);
    }
    Err(StatusCode::UNAUTHORIZED)
}
