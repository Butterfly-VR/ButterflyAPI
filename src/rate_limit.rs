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

// global rate limit, heavy endpoints should use a different limit function.
pub async fn rate_limit<
    const MINUTE_LIMIT: usize,
    const HOUR_LIMIT: usize,
    const DAY_LIMIT: usize,
>(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let addr = req
        .extract_parts::<ConnectInfo<SocketAddr>>()
        .await
        .unwrap();

    dbg!(addr);

    if let Some(addr_history) = state.request_history.read().await.get(&addr.ip()) {
        let mut requests_last_minute = 0;
        let mut requests_last_hour = 0;
        let mut requests_last_day = 0;

        let mut addr_history = addr_history.lock().await;

        let comparison_time = SystemTime::now();

        while let Some(x) = addr_history.iter().rev().next() {
            if dbg!(comparison_time.duration_since(*x)).unwrap_or_default()
                < Duration::from_hours(24)
            {
                break;
            }
            addr_history.pop_back();
        }
        addr_history.push_front(comparison_time);

        for req_time in addr_history.iter() {
            let time: Duration = comparison_time
                .duration_since(*req_time)
                .unwrap_or_default();
            if time < Duration::from_mins(1) {
                println!("request minute");
                requests_last_minute += 1;
            }
            if time < Duration::from_hours(1) {
                println!("request hour");
                requests_last_hour += 1;
            }
            println!("request day");
            requests_last_day += 1;
        }

        if requests_last_minute > MINUTE_LIMIT {
            println!("limit minute");
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        if requests_last_hour > HOUR_LIMIT {
            println!("limit hour");
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        if requests_last_day > DAY_LIMIT {
            println!("limit day");
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
