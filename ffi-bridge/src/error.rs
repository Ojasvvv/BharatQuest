use thiserror::Error;

#[derive(Error, Debug)]
pub enum SsrfError {
    #[error("SSRF blocked: target {0} resolves to the loopback block (127.0.0.0/8 or ::1)")]
    BlockedLoopback(String),

    #[error("SSRF blocked: target {0} resolves to AWS metadata (169.254.169.254/32)")]
    BlockedAwsMetadata(String),

    #[error("SSRF blocked: target {0} resolves to a private network (10.0.0.0/8, 172.16.0.0/12, or 192.168.0.0/16)")]
    BlockedPrivateNetwork(String),

    #[error("SSRF blocked: target {0} resolves to a unique local address (fc00::/7) or link-local address (fe80::/10)")]
    BlockedUniqueLocal(String),

    #[error("Network error: failed to resolve host {0}")]
    DnsResolutionFailed(String),

    #[error("Too many redirects (max 5 hops exceeded)")]
    TooManyRedirects,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Outbound fetch failed: {0}")]
    FetchFailed(String),

    #[error("Network fetch is disabled by host configuration")]
    FetchDisabled,
}
