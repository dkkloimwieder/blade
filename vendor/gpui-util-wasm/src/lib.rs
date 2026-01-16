//! WASM-compatible fork of gpui_util
//!
//! This crate provides utility functions for GPUI that work on both native and WASM targets.
//! Native-only features (fs, command, shell, archive) are not included in this minimal fork.

pub mod arc_cow;
pub mod http_stubs;
pub mod paths;
pub mod serde;
pub mod size;
pub mod time;

// Re-exports
pub use arc_cow::ArcCow;
pub use http_stubs::{HeaderValue, StatusCode, Uri, Url, http};

/// Defer execution of a closure until the returned guard is dropped.
pub fn defer<F: FnOnce()>(f: F) -> Deferred<F> {
    Deferred::new(f)
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

/// Post-increment helper - increments by 1 and returns previous value
pub fn post_inc(value: &mut usize) -> usize {
    let prev = *value;
    *value += 1;
    prev
}

/// A deferred operation that runs when dropped
pub struct Deferred<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Deferred<F> {
    /// Create a new deferred operation
    pub fn new(f: F) -> Self {
        Self(Some(f))
    }

    /// Abort the deferred operation without running it
    pub fn abort(mut self) {
        self.0.take();
    }
}

impl<F: FnOnce()> Drop for Deferred<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

/// Extension trait for TryFuture
pub trait TryFutureExt: std::future::Future {
    /// Log errors from a future
    fn log_err(self) -> LogErrFuture<Self>
    where
        Self: Sized,
    {
        LogErrFuture(self)
    }

    /// Log errors from a future with location tracking
    fn log_tracked_err(self, _location: core::panic::Location<'static>) -> LogErrFuture<Self>
    where
        Self: Sized,
    {
        // On WASM, location tracking is simplified - just log the error
        LogErrFuture(self)
    }
}

impl<F: std::future::Future> TryFutureExt for F {}

/// Future wrapper that logs errors
#[pin_project::pin_project]
pub struct LogErrFuture<F>(#[pin] F);

impl<T, E: std::fmt::Debug, F: std::future::Future<Output = Result<T, E>>> std::future::Future
    for LogErrFuture<F>
{
    type Output = Option<T>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        use std::task::Poll;
        let this = self.project();
        match this.0.poll(cx) {
            Poll::Ready(Ok(v)) => Poll::Ready(Some(v)),
            Poll::Ready(Err(e)) => {
                log::error!("{:?}", e);
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Measure the duration of a block of code
pub fn measure<R>(label: &str, f: impl FnOnce() -> R) -> R {
    // Use web_time::Instant which works on both native and WASM
    let start = web_time::Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    log::debug!("{}: {:?}", label, elapsed);
    result
}

/// Trigger a debug panic (only in debug mode)
#[macro_export]
macro_rules! debug_panic {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            panic!($($arg)*);
        } else {
            log::error!("debug_panic: {}", format!($($arg)*));
        }
    };
}

/// Macro for maybe expressions (returns early on None)
#[macro_export]
macro_rules! maybe {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => return None,
        }
    };
}
