//! Minimal stderr logger on an error/warning/info/debug scale.
//!
//! The threshold defaults to [`LogLevel::Warning`]: errors and warnings are
//! always shown, while info/debug diagnostics (summary stats, per-file timing,
//! parse diagnostics) only appear when the level is lowered with `--verbose`
//! (`-v` → info, `-vv` → debug). The threshold is a process-wide atomic so it
//! is safe to check from rayon worker threads.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Error = 0,
    Warning = 1,
    Info = 2,
    Debug = 3,
}

static THRESHOLD: AtomicU8 = AtomicU8::new(LogLevel::Warning as u8);

/// Set the log threshold; messages at or below this level are emitted.
pub fn set_level(level: LogLevel) {
    THRESHOLD.store(level as u8, Ordering::Relaxed);
}

/// The currently configured threshold.
pub fn level() -> LogLevel {
    match THRESHOLD.load(Ordering::Relaxed) {
        0 => LogLevel::Error,
        1 => LogLevel::Warning,
        2 => LogLevel::Info,
        _ => LogLevel::Debug,
    }
}

/// Emit `args` to stderr if `level` is at or below the configured threshold.
pub fn log(level: LogLevel, args: std::fmt::Arguments) {
    if level as u8 <= THRESHOLD.load(Ordering::Relaxed) {
        eprintln!("{}: {}", prefix(level), args);
    }
}

fn prefix(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "error",
        LogLevel::Warning => "warning",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
    }
}

#[macro_export]
macro_rules! error_log {
    ($($arg:tt)*) => { $crate::log::log($crate::log::LogLevel::Error, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! warn_log {
    ($($arg:tt)*) => { $crate::log::log($crate::log::LogLevel::Warning, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! info_log {
    ($($arg:tt)*) => { $crate::log::log($crate::log::LogLevel::Info, format_args!($($arg)*)) };
}

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => { $crate::log::log($crate::log::LogLevel::Debug, format_args!($($arg)*)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_roundtrip() {
        for lvl in [LogLevel::Error, LogLevel::Warning, LogLevel::Info, LogLevel::Debug] {
            set_level(lvl);
            assert_eq!(level(), lvl);
        }
        set_level(LogLevel::Warning);
    }

    #[test]
    fn test_threshold_ordering() {
        // The scale is ordered most-severe first; info is below warning.
        assert!(LogLevel::Error < LogLevel::Warning);
        assert!(LogLevel::Warning < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
    }
}
