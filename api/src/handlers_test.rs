use super::build_app;
use crate::state::AppState;
use crate::models::ExecuteRequest;
use apatheia_engine::RuntimePool;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::path::PathBuf;
use tower::ServiceExt;

async fn setup_app() -> axum::Router {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let base_dir = std::path::PathBuf::from(manifest_dir).parent().unwrap().join("wasm-runtimes");
    std::env::set_var("WASM_BINARY_DIR", base_dir);

    let pool = RuntimePool::init().await.unwrap();
    let state = AppState::new(pool);

    crate::build_app(state)
}

#[tokio::test]
async fn test_valid_execution_success() {
    let app = setup_app().await;

    let req_body = ExecuteRequest {
        request_id: "req-123".to_string(),
        language: "javascript".to_string(),
        code: "console.log('hello world');".to_string(),
        timeout_ms: 1000,
        memory_limit_mb: 128,
    };

    let request = Request::builder()
        .method("POST")
        .uri("/v1/execute")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(res["status"], "success");
    assert_eq!(res["request_id"], "req-123");
    assert_eq!(res["stdout"], "hello world\n");
    assert!(res["metrics"].is_object());
}

#[tokio::test]
async fn test_hallucinated_method_runtime_error() {
    let app = setup_app().await;

    let req_body = ExecuteRequest {
        request_id: "req-err".to_string(),
        language: "javascript".to_string(),
        code: "[1,2].mapIsCool(x => x)".to_string(),
        timeout_ms: 1000,
        memory_limit_mb: 128,
    };

    let request = Request::builder()
        .method("POST")
        .uri("/v1/execute")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK); // Still 200 OK because it successfully completed the request and returned the structured error shape

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(res["status"], "runtime_error");
    assert_eq!(res["request_id"], "req-err");
    assert_eq!(res["error_telemetry"]["type"], "RuntimeError");

    assert!(res["llm_feedback_prompt"]["content"].as_str().unwrap().contains("Execution failed: RuntimeError"));
}

#[tokio::test]
async fn test_timeout_rejection() {
    let app = setup_app().await;

    let req_body = ExecuteRequest {
        request_id: "req-to".to_string(),
        language: "javascript".to_string(),
        code: "while(true){}".to_string(), // Actually this hits fuel exhaustion first since fuel limits are enforced. But let's test input validation timeout first.
        timeout_ms: 99999999, // Over MAX_TIMEOUT_MS
        memory_limit_mb: 128,
    };

    let request = Request::builder()
        .method("POST")
        .uri("/v1/execute")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_infinite_loop_fuel_trap_rejection() {
    let app = setup_app().await;

    let req_body = ExecuteRequest {
        request_id: "req-fuel".to_string(),
        language: "javascript".to_string(),
        code: "while(true){}".to_string(),
        timeout_ms: 5000,
        memory_limit_mb: 128,
    };

    let request = Request::builder()
        .method("POST")
        .uri("/v1/execute")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let res: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(res["status"], "rejected");
    assert_eq!(res["reason"], "out_of_fuel");
    assert!(res["metrics"].is_object());
    assert!(res["metrics"]["fuel_consumed"].as_u64().unwrap() > 0);
}
