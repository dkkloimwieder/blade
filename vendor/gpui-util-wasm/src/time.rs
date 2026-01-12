//! Time utilities that work on both native and WASM

use std::time::Duration;

/// Format a duration as a human-readable string
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if secs >= 3600 {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{}h {}m", hours, mins)
    } else if secs >= 60 {
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}m {}s", mins, secs)
    } else if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else if millis > 0 {
        format!("{}ms", millis)
    } else {
        format!("{}µs", duration.subsec_micros())
    }
}

/// A simple stopwatch for measuring elapsed time
pub struct Stopwatch {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    start_ms: f64,
}

impl Stopwatch {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
            #[cfg(target_arch = "wasm32")]
            start_ms: 0.0, // Would use performance.now() in real impl
        }
    }

    pub fn elapsed(&self) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Placeholder - real impl would use web_sys::window().performance()
            Duration::from_millis(0)
        }
    }

    pub fn restart(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start = std::time::Instant::now();
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.start_ms = 0.0;
        }
    }
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}
