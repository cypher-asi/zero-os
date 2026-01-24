//! Network types for Zero OS
//!
//! This crate provides HTTP request/response types for network operations
//! mediated by the Network Service.
//!
//! # Architecture
//!
//! Network access in Zero OS flows through the Network Service:
//!
//! ```text
//! Process (e.g. Identity Service)
//!        │
//!        │ IPC (MSG_NET_REQUEST)
//!        ▼
//! ┌─────────────────┐
//! │ Network Service │  ◄── Mediates all HTTP requests
//! │   (Process)     │
//! └────────┬────────┘
//!          │
//!          │ SYS_NETWORK_FETCH syscall (returns request_id)
//!          ▼
//! ┌─────────────────┐
//! │   Supervisor    │
//! └────────┬────────┘
//!          │
//!          │ ZosNetwork.startFetch()
//!          ▼
//! ┌─────────────────┐
//! │  Browser fetch  │
//! └────────┬────────┘
//!          │
//!          │ Promise resolves
//!          ▼
//! ┌─────────────────┐
//! │   Supervisor    │  ◄── onNetworkResult()
//! └────────┬────────┘
//!          │
//!          │ IPC (MSG_NET_RESULT)
//!          ▼
//! ┌─────────────────┐
//! │ Network Service │  ◄── Routes response to caller
//! └─────────────────┘
//! ```

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// =============================================================================
// HTTP Method
// =============================================================================

/// HTTP request method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    /// HTTP GET
    Get,
    /// HTTP POST
    Post,
    /// HTTP PUT
    Put,
    /// HTTP DELETE
    Delete,
    /// HTTP PATCH
    Patch,
    /// HTTP HEAD
    Head,
    /// HTTP OPTIONS
    Options,
}

impl HttpMethod {
    /// Convert to lowercase string for fetch API.
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

// =============================================================================
// HTTP Request
// =============================================================================

/// HTTP request to be sent via the Network Service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpRequest {
    /// HTTP method (GET, POST, etc.)
    pub method: HttpMethod,
    /// Target URL
    pub url: String,
    /// Request headers as key-value pairs
    pub headers: Vec<(String, String)>,
    /// Request body (optional)
    pub body: Option<Vec<u8>>,
    /// Request timeout in milliseconds
    pub timeout_ms: u32,
    /// Caller PID (set by Network Service)
    #[serde(default)]
    pub caller_pid: u32,
    /// Request ID for tracking async response
    #[serde(default)]
    pub request_id: u32,
}

impl HttpRequest {
    /// Create a new GET request.
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: HttpMethod::Get,
            url: url.into(),
            headers: Vec::new(),
            body: None,
            timeout_ms: 30_000,
            caller_pid: 0,
            request_id: 0,
        }
    }

    /// Create a new POST request.
    pub fn post(url: impl Into<String>) -> Self {
        Self {
            method: HttpMethod::Post,
            url: url.into(),
            headers: Vec::new(),
            body: None,
            timeout_ms: 30_000,
            caller_pid: 0,
            request_id: 0,
        }
    }

    /// Set request body.
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    /// Set JSON body and Content-Type header.
    pub fn with_json_body(mut self, body: Vec<u8>) -> Self {
        self.headers
            .push(("Content-Type".into(), "application/json".into()));
        self.body = Some(body);
        self
    }

    /// Add a header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set timeout in milliseconds.
    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set authorization bearer token.
    pub fn with_bearer_token(self, token: impl Into<String>) -> Self {
        self.with_header("Authorization", alloc::format!("Bearer {}", token.into()))
    }
}

// =============================================================================
// HTTP Response
// =============================================================================

/// HTTP response from the Network Service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    /// Result of the HTTP request
    pub result: Result<HttpSuccess, NetworkError>,
}

impl HttpResponse {
    /// Create a successful response.
    pub fn ok(status: u16, headers: Vec<(String, String)>, body: Vec<u8>) -> Self {
        Self {
            result: Ok(HttpSuccess {
                status,
                headers,
                body,
            }),
        }
    }

    /// Create an error response.
    pub fn err(error: NetworkError) -> Self {
        Self { result: Err(error) }
    }

    /// Check if the response was successful (2xx status).
    pub fn is_success(&self) -> bool {
        match &self.result {
            Ok(success) => (200..300).contains(&success.status),
            Err(_) => false,
        }
    }
}

/// Successful HTTP response data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpSuccess {
    /// HTTP status code (200, 404, etc.)
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: Vec<u8>,
}

// =============================================================================
// Network Error
// =============================================================================

/// Errors that can occur during network operations.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkError {
    /// Network access denied by policy
    PolicyDenied,
    /// Failed to establish connection
    ConnectionFailed,
    /// Request timed out
    Timeout,
    /// Invalid URL format
    InvalidUrl,
    /// DNS resolution failed
    DnsError,
    /// SSL/TLS error
    TlsError,
    /// Network service not available
    ServiceUnavailable,
    /// Other error with description
    Other(String),
}

impl NetworkError {
    /// Convert to a user-friendly error message.
    pub fn message(&self) -> &str {
        match self {
            NetworkError::PolicyDenied => "Network access denied",
            NetworkError::ConnectionFailed => "Failed to connect",
            NetworkError::Timeout => "Request timed out",
            NetworkError::InvalidUrl => "Invalid URL",
            NetworkError::DnsError => "DNS resolution failed",
            NetworkError::TlsError => "SSL/TLS error",
            NetworkError::ServiceUnavailable => "Network service unavailable",
            NetworkError::Other(msg) => msg,
        }
    }
}

// =============================================================================
// IPC Message Types
// =============================================================================

/// Request sent to Network Service via IPC.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetRequest {
    /// The HTTP request to perform
    pub request: HttpRequest,
}

/// Response from Network Service via IPC.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetResponse {
    /// Request ID that this response corresponds to
    pub request_id: u32,
    /// The HTTP response
    pub response: HttpResponse,
}

// =============================================================================
// Result Types for Network Operations
// =============================================================================

/// Network result types for MSG_NET_RESULT IPC messages.
pub mod result {
    /// Request succeeded, response body follows
    pub const NET_OK: u8 = 0;
    /// Request failed with error
    pub const NET_ERROR: u8 = 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_http_request_builder() {
        let req = HttpRequest::get("https://api.example.com/data")
            .with_header("Accept", "application/json")
            .with_bearer_token("test-token")
            .with_timeout(5000);

        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.url, "https://api.example.com/data");
        assert_eq!(req.timeout_ms, 5000);
        assert_eq!(req.headers.len(), 2);
    }

    #[test]
    fn test_http_response() {
        let resp = HttpResponse::ok(200, vec![], b"hello".to_vec());
        assert!(resp.is_success());

        let resp = HttpResponse::err(NetworkError::Timeout);
        assert!(!resp.is_success());
    }
}
