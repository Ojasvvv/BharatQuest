//! FFI bridge error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FfiBridgeError {
    #[error("SSRF blocked: target {url} resolves to a private/loopback address")]
    SsrfBlocked { url: String },

    #[error("Outbound fetch failed: {0}")]
    FetchFailed(#[from] reqwest::Error),

    #[error("Fetch response too large: {size_bytes} bytes exceeds limit of {limit_bytes} bytes")]
    ResponseTooLarge { size_bytes: u64, limit_bytes: u64 },

    #[error("Fetch timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("URL parse error: {0}")]
    UrlParse(String),
}
