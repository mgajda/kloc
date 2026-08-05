//! Minimal stderr logger on an error/warning/info/debug scale.
//!
//! Default threshold is [`LogLevel::Warning`]; `-v` lowers it to info, `-vv`
//! to debug. The threshold is a process-wide atomic, so it is safe to check
//! from rayon worker threads.

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
        for lvl in [
            LogLevel::Error,
            LogLevel::Warning,
            LogLevel::Info,
            LogLevel::Debug,
        ] {
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

    #[test]
    fn prefix_matches_level() {
        assert_eq!(prefix(LogLevel::Error), "error");
        assert_eq!(prefix(LogLevel::Warning), "warning");
        assert_eq!(prefix(LogLevel::Info), "info");
        assert_eq!(prefix(LogLevel::Debug), "debug");
    }

    #[test]
    fn log_emits_at_or_below_threshold() {
        set_level(LogLevel::Debug);
        log(LogLevel::Error, format_args!("error"));
        log(LogLevel::Warning, format_args!("warning"));
        log(LogLevel::Info, format_args!("info"));
        log(LogLevel::Debug, format_args!("debug"));
        error_log!("macro error");
        warn_log!("macro warn");
        info_log!("macro info");
        debug_log!("macro debug");
        set_level(LogLevel::Warning);
    }

    #[test]
    fn log_suppressed_above_threshold() {
        set_level(LogLevel::Error);
        log(LogLevel::Info, format_args!("hidden info"));
        info_log!("hidden macro info");
        set_level(LogLevel::Warning);
    }
}
