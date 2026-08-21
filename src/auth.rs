use crate::{ApiError, AppState, schema::instances};
use crate::{RateLimitInfo, schema};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use schema::tokens::dsl::{expires, token, tokens, user};
use std::sync::atomic::AtomicU64;
use std::time::Instant;
use std::{sync::Arc, time::SystemTime};
use tracing::{debug, trace};
use uuid::Uuid;

const RATE_LIMIT_WINDOW: std::time::Duration = std::time::Duration::from_hours(1);
const RATE_LIMIT_THRESHOLD: u64 = 2000;

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
        .filter(expires.gt(SystemTime::now()))
        .first::<Uuid>(&mut conn)
        .await
        .optional()
    {
        let rate_limit_info = state.user_rate_limits.read().await;

        // only acquire write lock if we need to insert
        if let Some(info) = rate_limit_info.get(&user_id) {
            if info.next_reset < Instant::now() {
                drop(rate_limit_info);
                state
                    .user_rate_limits
                    .write()
                    .await
                    .insert(user_id, RateLimitInfo::new(RATE_LIMIT_WINDOW));
            } else {
                if AtomicU64::load(&info.total_requests, std::sync::atomic::Ordering::Relaxed)
                    >= RATE_LIMIT_THRESHOLD
                {
                    debug!("rate limit exceeded for user {}", user_id);
                    return Err(ApiError::WithCode(StatusCode::TOO_MANY_REQUESTS));
                }

                AtomicU64::fetch_add(
                    &info.total_requests,
                    1,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
        } else {
            drop(rate_limit_info);
            state
                .user_rate_limits
                .write()
                .await
                .insert(user_id, RateLimitInfo::new(RATE_LIMIT_WINDOW));
        }

        req.extensions_mut().insert(user_id);
        trace!(
            "authenticated user {:?} for access to {:?}",
            user_id,
            req.uri()
        );
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
        trace!(
            "authenticated instance {:?} for access to {:?}",
            instance_id,
            req.uri()
        );
        return Ok(next.run(req).await);
    }
    debug!("unauthorized request to {:?}", req.uri());
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
    if let Ok(Some(instance_id)) = instances::table
        .select(instances::id)
        .filter(instances::server_token.eq(header_token))
        .first::<Uuid>(&mut conn)
        .await
        .optional()
    {
        req.extensions_mut().insert(instance_id);
        trace!(
            "authenticated instance {:?} for access to {:?}",
            instance_id,
            req.uri()
        );
        return Ok(next.run(req).await);
    }
    debug!(
        "unauthorized request to instance api endpoint {:?}",
        req.uri()
    );
    Err(ApiError::WithCode(StatusCode::UNAUTHORIZED))
}
