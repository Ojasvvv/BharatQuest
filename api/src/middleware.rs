use axum::{
    extract::{State, Request},
    middleware::Next,
    response::{IntoResponse, Response},
    http::{StatusCode, header},
    Json,
};
use serde_json::json;
use std::num::NonZeroU32;
use governor::{RateLimiter, Quota, clock::Clock};
use crate::state::AppState;

pub async fn auth_and_rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    // 1. Extract API Key
    let api_key = match req.headers().get("X-API-Key") {
        Some(key) => key.to_str().unwrap_or("").to_string(),
        None => {
            let body = Json(json!({
                "status": "error",
                "message": "Missing X-API-Key header"
            }));
            return Err((StatusCode::UNAUTHORIZED, body).into_response());
        }
    };

    // 2. Validate API Key
    if !state.valid_api_keys.contains(&api_key) {
        let body = Json(json!({
            "status": "error",
            "message": "Invalid API Key"
        }));
        return Err((StatusCode::UNAUTHORIZED, body).into_response());
    }

    // 3. Rate Limiting (5 requests per second per key, burst of 10)
    let check_result = {
        let limiter = state.rate_limiters.entry(api_key.clone()).or_insert_with(|| {
            let quota = Quota::per_second(NonZeroU32::new(5).unwrap())
                .allow_burst(NonZeroU32::new(10).unwrap());
            RateLimiter::direct(quota)
        });
        limiter.check()
    };

    if let Err(not_until) = check_result {
        let wait_time = not_until.wait_time_from(governor::clock::DefaultClock::default().now());
        
        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "status": "error",
                "message": "Rate limit exceeded",
                "error_code": "rate_limit_exceeded"
            }))
        ).into_response();
        
        let mut wait_secs = wait_time.as_secs_f64().ceil() as u64;
        if wait_secs == 0 {
            wait_secs = 1;
        }
        
        response.headers_mut().insert(
            header::RETRY_AFTER,
            wait_secs.to_string().parse().unwrap(),
        );
        
        return Err(response);
    }

    // 4. Pass to next handler
    Ok(next.run(req).await)
}
