//! WASM-compatible fork of gpui_util
//!
//! This crate provides utility functions for GPUI that work on both native and WASM targets.
//! Native-only features (fs, command, shell, archive) are not included in this minimal fork.

pub mod arc_cow;
pub mod paths;
pub mod serde;
pub mod size;
pub mod time;

// Re-exports
pub use arc_cow::ArcCow;

/// Defer execution of a closure until the returned guard is dropped.
pub fn defer<F: FnOnce()>(f: F) -> impl Drop {
    struct Defer<F: FnOnce()>(Option<F>);
    impl<F: FnOnce()> Drop for Defer<F> {
        fn drop(&mut self) {
            if let Some(f) = self.0.take() {
                f();
            }
        }
    }
    Defer(Some(f))
}

/// Extension trait for Result types to log errors
pub trait ResultExt<T> {
    fn log_err(self) -> Option<T>;
    fn warn_on_err(self) -> Option<T>;
}

impl<T, E: std::fmt::Debug> ResultExt<T> for Result<T, E> {
    fn log_err(self) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                log::error!("{:?}", e);
                None
            }
        }
    }

    fn warn_on_err(self) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                log::warn!("{:?}", e);
                None
            }
        }
    }
}

/// Extension trait for Option types
pub trait OptionExt<T> {
    fn log_none(self, msg: &str) -> Option<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn log_none(self, msg: &str) -> Option<T> {
        if self.is_none() {
            log::warn!("{}", msg);
        }
        self
    }
}

/// Truncate a string to a maximum number of characters
pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}

/// Truncate a string from the start
pub fn truncate_and_remove_front(s: &str, max_chars: usize) -> &str {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s;
    }
    let skip = char_count - max_chars;
    match s.char_indices().nth(skip) {
        None => "",
        Some((idx, _)) => &s[idx..],
    }
}

/// Post-increment helper
pub fn post_inc<T: std::ops::AddAssign<T> + Copy>(value: &mut T, delta: T) -> T {
    let prev = *value;
    *value += delta;
    prev
}
