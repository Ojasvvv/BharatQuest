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
    let language = match &req.language {
        Some(l) => l,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "missing_field",
                    "field": "language"
                }))
            ).into_response();
        }
    };
    let lang_str = language.to_lowercase();
    let runtime = match lang_str.as_str() {
        "javascript" | "js" => apatheia_engine::Runtime::JavaScript,
        "python" | "py" => apatheia_engine::Runtime::Python,
        _ => return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "unsupported_language",
                "supported": ["javascript", "python"]
            }))
        ).into_response(),
    };

    if let Some(parent_id) = &req.parent_request_id {
        let mut counts = state.retry_counts.lock().unwrap();
        let entry = counts.entry(parent_id.clone()).or_insert((0, std::time::Instant::now()));
        entry.0 += 1;
        entry.1 = std::time::Instant::now();
        if entry.0 > 3 {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({
                    "status": "error",
                    "message": "Self-healing loop exceeded maximum retry attempts.",
                    "error_code": "max_iterations_exceeded",
                    "request_id": req.request_id,
                    "parent_request_id": parent_id,
                    "max_iterations": 3
                }))
            ).into_response();
        }
    }

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
                    language: lang_str.clone(),
                    status: "rejected".to_string(),
                    metrics: m.clone(),
                });
            }
            return Json(ExecuteResponse::Rejected {
                request_id: req.request_id,
                language: lang_str,
                reason,
                metrics,
            })
            .into_response();
        }
        Err(_) => {
            // tokio::time::timeout elapsed
            return Json(ExecuteResponse::Rejected {
                request_id: req.request_id,
                language: lang_str,
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
        if let Some(parent_id) = &req.parent_request_id {
            state.retry_counts.lock().unwrap().remove(parent_id);
        }

        let _ = state.metrics_tx.send(StreamEvent {
            request_id: req.request_id.clone(),
            language: lang_str.clone(),
            status: "success".to_string(),
            metrics: result.metrics.clone(),
        });

        Json(ExecuteResponse::Success {
            request_id: req.request_id,
            language: lang_str,
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
            language: lang_str.clone(),
            status: "runtime_error".to_string(),
            metrics: result.metrics.clone(),
        });

        Json(ExecuteResponse::RuntimeError {
            request_id: req.request_id,
            language: lang_str,
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

#[derive(serde::Serialize)]
struct RuntimeInfo {
    id: String,
    label: String,
    status: String,
    wasm_binary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime_notes: Option<String>,
}

#[derive(serde::Serialize)]
struct RuntimesResponse {
    runtimes: Vec<RuntimeInfo>,
}

pub async fn runtimes_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut runtimes = vec![];
    
    runtimes.push(RuntimeInfo {
        id: "javascript".to_string(),
        label: "JavaScript (QuickJS)".to_string(),
        status: "ready".to_string(),
        wasm_binary: "quickjs.wasm".to_string(),
        runtime_notes: None,
    });
    
    let has_python = state.pool.python_module.is_some();
    
    runtimes.push(RuntimeInfo {
        id: "python".to_string(),
        label: "Python (MicroPython)".to_string(),
        status: if has_python { "ready".to_string() } else { "unavailable".to_string() },
        wasm_binary: "micropython-wasi.wasm".to_string(),
        runtime_notes: if has_python { 
            Some("MicroPython 1.x — standard library subset only".to_string()) 
        } else { 
            Some("Not implemented".to_string()) 
        },
    });
    
    Json(RuntimesResponse { runtimes })
}
