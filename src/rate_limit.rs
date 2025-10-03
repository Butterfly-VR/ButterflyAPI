use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::trace;

pub async fn check_rate_limits(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    // database access goes here
    trace!("checked limits for {:#?}", req.uri());
    Ok(next.run(req).await)
}
