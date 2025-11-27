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
use std::sync::Arc;
use tracing::{trace, warn};
use uuid::Uuid;

use crate::schema;

pub async fn check_auth(
    state: State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    trace!("reached auth layer");
    let conn = state.pool.get().await;
    if conn.is_err() {
        warn!("failed to aquire db connection");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut conn = conn.unwrap();
    let header_token = req
        .headers()
        .get("token")
        .map(|x| x.as_bytes())
        .unwrap_or_default();
    if let Some(Some(user_id)) = tokens
        .select(user)
        .filter(token.eq(header_token))
        .first::<Uuid>(&mut conn)
        .await
        .optional()
        .ok()
    {
        req.extensions_mut().insert(user_id);
        return Ok(next.run(req).await);
    }
    Err(StatusCode::UNAUTHORIZED)
}
