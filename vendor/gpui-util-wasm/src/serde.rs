//! Serde utilities

use serde::{Deserialize, Serialize};

/// Serialize a value to a pretty-printed JSON string
pub fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

/// Deserialize from a JSON string, with better error messages
pub fn from_json<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, serde_json::Error> {
    serde_json::from_str(s)
}

/// Default value helper for serde
pub fn default<T: Default>() -> T {
    T::default()
}
