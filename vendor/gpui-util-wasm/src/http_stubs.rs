//! Minimal HTTP stubs for WASM
//!
//! These provide the same API surface as http_client but without actual HTTP functionality.
//! On WASM, HTTP requests will use the browser's fetch API instead.

use std::str::FromStr;

/// A stub URI type for WASM
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Uri(String);

impl Uri {
    /// Check if a string looks like a valid URI
    pub fn from_str(s: &str) -> Result<Self, UriError> {
        // Simple heuristic: check if it starts with a valid scheme
        if s.starts_with("http://")
            || s.starts_with("https://")
            || s.starts_with("file://")
            || s.starts_with("data:")
        {
            Ok(Uri(s.to_string()))
        } else {
            Err(UriError)
        }
    }
}

impl FromStr for Uri {
    type Err = UriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uri::from_str(s)
    }
}

#[derive(Debug)]
pub struct UriError;

impl std::fmt::Display for UriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid URI")
    }
}

impl std::error::Error for UriError {}

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(u16);

impl StatusCode {
    pub const OK: StatusCode = StatusCode(200);
    pub const NOT_FOUND: StatusCode = StatusCode(404);
    pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode(500);

    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn from_u16(code: u16) -> Result<Self, ()> {
        if code < 600 {
            Ok(StatusCode(code))
        } else {
            Err(())
        }
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// HTTP header value stub
#[derive(Debug, Clone)]
pub struct HeaderValue(String);

impl HeaderValue {
    pub fn from_str(s: &str) -> Result<Self, ()> {
        Ok(HeaderValue(s.to_string()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// Stub for HTTP module
pub mod http {
    pub use super::HeaderValue;
}

/// URL type (re-export or stub)
pub type Url = String;
