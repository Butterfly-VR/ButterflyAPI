use crate::AppState;
use axum::{
    RequestExt,
    body::Body,
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::{collections::VecDeque, net::SocketAddr, time::SystemTime};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

async fn check_limit_inner(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
    minute_limit: usize,
    hour_limit: usize,
    day_limit: usize,
) -> Result<Response, StatusCode> {
    let addr = req
        .extract_parts::<ConnectInfo<SocketAddr>>()
        .await
        .unwrap();
    if let Some(addr_history) = state.request_history.read().await.get(&addr.ip()) {
        let mut requests_last_minute = 0;
        let mut requests_last_hour = 0;
        let mut requests_last_day = 0;

        let mut addr_history = addr_history.lock().await;

        addr_history.pop_back();
        addr_history.push_front(SystemTime::now());

        let comparison_time = SystemTime::now();
        for req_time in addr_history.iter() {
            let time: Duration = comparison_time
                .duration_since(*req_time)
                .unwrap_or_default();
            if time > Duration::from_hours(24) {
                break;
            }
            if time > Duration::from_hours(1) {
                requests_last_day += 1;
                continue;
            }
            if time > Duration::from_mins(1) {
                requests_last_hour += 1;
                continue;
            }
            requests_last_minute += 1;
        }

        if requests_last_minute > minute_limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        if requests_last_hour > hour_limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        if requests_last_day > day_limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
    } else {
        state.request_history.write().await.insert(
            addr.ip(),
            Mutex::new(VecDeque::from_iter(Some(SystemTime::now()).into_iter())),
        );
    }
    Ok(next.run(req).await)
}

pub async fn rate_limit_basic(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // some abritary initial limits
    // 2 req per sec for a minute, half that for an hour, quater that for a day
    const MINUTE_RATE_LIMIT: usize = 120;
    const HOUR_RATE_LIMIT: usize = MINUTE_RATE_LIMIT * 30;
    const DAY_RATE_LIMIT: usize = HOUR_RATE_LIMIT * 12;
    check_limit_inner(
        State(state),
        req,
        next,
        MINUTE_RATE_LIMIT,
        HOUR_RATE_LIMIT,
        DAY_RATE_LIMIT,
    )
    .await
}
