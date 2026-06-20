use apatheia_telemetry::ExecutionMetrics;
use serde::{Deserialize, Serialize};

pub const MAX_TIMEOUT_MS: u64 = 10_000;
pub const MAX_MEMORY_MB: usize = 256;

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecuteRequest {
    pub request_id: String,
    pub language: String,
    pub code: String,
    pub timeout_ms: u64,
    pub memory_limit_mb: usize,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExecuteResponse {
    Success {
        request_id: String,
        stdout: String,
        metrics: ExecutionMetrics,
    },
    RuntimeError {
        request_id: String,
        metrics: ExecutionMetrics,
        error_telemetry: ErrorTelemetry,
        llm_feedback_prompt: LlmFeedbackPrompt,
    },
    Rejected {
        request_id: String,
        reason: RejectReason,
        #[serde(skip_serializing_if = "Option::is_none")]
        metrics: Option<ExecutionMetrics>,
    },
}

#[derive(Debug, Serialize)]
pub struct ErrorTelemetry {
    #[serde(rename = "type")]
    pub error_type: String,
    pub trace: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct LlmFeedbackPrompt {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    Timeout,
    OutOfMemory,
    OutOfFuel,
    InvalidInput,
}
