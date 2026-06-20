use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::time::Duration;
use apatheia_engine::error::EngineError;

use crate::models::{
    ErrorTelemetry, ExecuteRequest, ExecuteResponse, LlmFeedbackPrompt, RejectReason,
    MAX_MEMORY_MB, MAX_TIMEOUT_MS,
};
use crate::state::{AppState, StreamEvent};

fn parse_python_exception(stderr: &str) -> (String, String) {
    let last_line = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .last()
        .unwrap_or(stderr.trim());

    if let Some(colon_pos) = last_line.find(": ") {
        let exc_type = last_line[..colon_pos].trim().to_string();
        let message  = last_line[colon_pos + 2..].trim().to_string();
        (exc_type, message)
    } else {
        ("RuntimeError".to_string(), last_line.trim().to_string())
    }
}

pub async fn execute_handler(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> impl IntoResponse {
    // 1. Strict Input Validation
    if req.timeout_ms > crate::models::MAX_TIMEOUT_MS {
        return (
            StatusCode::BAD_REQUEST,
            format!("Timeout must be <= {} ms", crate::models::MAX_TIMEOUT_MS),
        )
            .into_response();
    }
    if req.memory_limit_mb > crate::models::MAX_MEMORY_MB {
        return (
            StatusCode::BAD_REQUEST,
            format!("Memory limit must be <= {} MB", crate::models::MAX_MEMORY_MB),
        )
            .into_response();
    }
    let lang_str = req.language.to_lowercase();
    let runtime = match lang_str.as_str() {
        "javascript" | "js" => apatheia_engine::Runtime::JavaScript,
        "python" | "py" => apatheia_engine::Runtime::Python,
        _ => return (StatusCode::BAD_REQUEST, "Unsupported language").into_response(),
    };

    let execute_future = state.pool.execute(
        &runtime,
        &req.code,
        50_000_000,
        req.timeout_ms,
        req.memory_limit_mb as u32,
    );
    let result_or_timeout = tokio::time::timeout(Duration::from_millis(req.timeout_ms), execute_future).await;

    let result = match result_or_timeout {
        Ok(Ok(res)) => res,
        Ok(Err(err)) => {
            let (reason, metrics) = match err {
                EngineError::FuelExhausted { metrics, .. } => (RejectReason::OutOfFuel, Some(metrics)),
                EngineError::WallClockTimeout { metrics, .. } => (RejectReason::Timeout, Some(metrics)),
                EngineError::MemoryLimitExceeded { metrics, .. } => (RejectReason::OutOfMemory, Some(metrics)),
                EngineError::RuntimeUnavailable(lang) => {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        axum::Json(serde_json::json!({
                            "error": "runtime_unavailable",
                            "runtime": lang
                        }))
                    ).into_response();
                }
                _ => (RejectReason::InvalidInput, None),
            };
            if let Some(ref m) = metrics {
                let _ = state.metrics_tx.send(StreamEvent {
                    request_id: req.request_id.clone(),
                    status: "rejected".to_string(),
                    metrics: m.clone(),
                });
            }
            return Json(ExecuteResponse::Rejected {
                request_id: req.request_id,
                reason,
                metrics,
            })
            .into_response();
        }
        Err(_) => {
            // tokio::time::timeout elapsed
            return Json(ExecuteResponse::Rejected {
                request_id: req.request_id,
                reason: RejectReason::Timeout,
                metrics: None, // Missing full context here since tokio timed out early
            })
            .into_response();
        }
    };

    let final_status = if result.status_code == 0 {
        "success"
    } else {
        "runtime_error"
    };

    // 2. Map ExecutionResult to JSON Output Contract
    if final_status == "success" {
        let _ = state.metrics_tx.send(StreamEvent {
            request_id: req.request_id.clone(),
            status: "success".to_string(),
            metrics: result.metrics.clone(),
        });

        Json(ExecuteResponse::Success {
            request_id: req.request_id,
            stdout: result.stdout,
            metrics: result.metrics,
        })
        .into_response()
    } else {
        // Map runtime/parse errors
        let js_err = result.error.unwrap_or_else(|| apatheia_engine::error::JsError {
            error_type: apatheia_engine::error::JsErrorType::Runtime,
            message: "Unknown error".to_string(),
            stack_trace: None,
        });

        let is_python = lang_str.as_str() == "python" || lang_str.as_str() == "py";
        let (error_type_str, message, trace) = if is_python {
            let (ex_name, msg) = parse_python_exception(&js_err.message);
            (ex_name, msg, Some(js_err.message.clone()))
        } else {
            let type_str = match js_err.error_type {
                apatheia_engine::error::JsErrorType::Runtime => "RuntimeError".to_string(),
                apatheia_engine::error::JsErrorType::Parse => "SyntaxError".to_string(),
            };
            (type_str, js_err.message.clone(), js_err.stack_trace.clone())
        };

        let llm_content = format!(
            "Execution failed: {}: {}. Review your code and provide a corrected script.",
            error_type_str, message
        );

        let _ = state.metrics_tx.send(StreamEvent {
            request_id: req.request_id.clone(),
            status: "runtime_error".to_string(),
            metrics: result.metrics.clone(),
        });

        Json(ExecuteResponse::RuntimeError {
            request_id: req.request_id,
            metrics: result.metrics,
            error_telemetry: ErrorTelemetry {
                error_type: error_type_str,
                trace,
                message,
            },
            llm_feedback_prompt: LlmFeedbackPrompt {
                role: "system".to_string(),
                content: llm_content,
            },
        })
        .into_response()
    }
}

pub async fn stream_metrics_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.metrics_tx.subscribe();
    while let Ok(metrics) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&metrics) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }
}

pub async fn runtimes_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut runtimes = vec![];
    let mut has_python = false;
    
    // QuickJS
    runtimes.push(serde_json::json!({
        "language": "javascript",
        "status": "ready",
        "runtime_notes": "JavaScript (QuickJS)"
    }));
    
    if state.pool.python_module.is_some() {
        has_python = true;
    }
    
    runtimes.push(serde_json::json!({
        "language": "python",
        "status": if has_python { "ready" } else { "unavailable" },
        "runtime_notes": if has_python { "MicroPython 1.x — standard library subset only" } else { "Not implemented" }
    }));
    
    Json(runtimes)
}
