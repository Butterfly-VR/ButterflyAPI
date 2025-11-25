use crate::AppState;
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
use tokio::task::yield_now;
use uuid::Uuid;

use crate::schema;

pub async fn check_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut conn;
    // poor man's .get_async().await
    loop {
        if let Some(c) = state.pool.try_get() {
            conn = c;
            break;
        }
        yield_now();
    }
    let header_token = req
        .headers()
        .get("token")
        .map(|x| x.as_bytes())
        .unwrap_or_default();
    if let Some(user_id) = tokens
        .select(user)
        .filter(token.eq(header_token))
        .load::<Uuid>(&mut conn)
        .optional()
        .ok()
    {
        req.extensions_mut().insert(user_id);
        return Ok(next.run(req).await);
    }
    Err(StatusCode::UNAUTHORIZED)
}
