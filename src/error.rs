// SPDX-License-Identifier: GPL-3.0-only
//! Error types for the application
//!
//! Provides comprehensive error handling with proper error types for all
//! failure modes in the application.

use thiserror::Error;

/// Main application error type
#[derive(Error, Debug)]
#[allow(dead_code)] // Comprehensive error types for future use
pub enum AppError {
    /// Failed to initialize a display
    #[error("Failed to initialize display {id}: {reason}")]
    DisplayInit { id: String, reason: String },

    /// DDC/CI communication error
    #[error("DDC/CI communication error on display {id}: {source}")]
    DdcCi {
        id: String,
        #[source]
        source: anyhow::Error,
    },

    /// Apple HID communication error
    #[cfg(feature = "apple-hid-displays")]
    #[error("Apple HID communication error on display {id}: {reason}")]
    AppleHid { id: String, reason: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// D-Bus error (for daemon communication)
    #[cfg(feature = "brightness-sync-daemon")]
    #[error("D-Bus error: {0}")]
    DBus(#[from] zbus::Error),

    /// Display not found in manager
    #[error("Display {0} not found")]
    DisplayNotFound(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Task join error
    #[error("Task join error: {0}")]
    TaskJoin(String),
}

/// Result type alias for AppError
pub type Result<T> = std::result::Result<T, AppError>;
