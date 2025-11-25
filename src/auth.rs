use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::trace;

pub async fn check_auth(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    trace!("authing for {:#?}", req.uri());
    // requires the http crate to get the header name
    if req
        .headers()
        .get("token")
        .unwrap_or(&HeaderValue::from_static(""))
        != "[1-2-3-4-5-6-7-8-1-2-3-4-5-6-7-8-1-2-3-4-5-6-7-8-1-2-3-4-5-6-7-8]"
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
