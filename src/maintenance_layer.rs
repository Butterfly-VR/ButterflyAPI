use crate::{ApiError, ErrorInfo};
use axum::Json;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::env;
use std::sync::LazyLock;

// once the config file has been updated with maintenance info the deployment must be restarted
static INFO: LazyLock<String> = LazyLock::new(|| env::var("MAINTENANCE_INFO").unwrap_or_default());

pub async fn maintenance_layer(req: Request<Body>, next: Next) -> Result<Response, ApiError> {
    if !INFO.is_empty() {
        return Err(ApiError::WithResponse(
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorInfo {
                error_code: crate::ErrorCode::DownForMaintenance,
                error_message: Some(INFO.clone()),
            }),
        ));
    }

    return Ok(next.run(req).await);
}
