use serde::{Deserialize, Serialize};
use std::fmt;

/// Custom error types for the Cap application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapError {
    // Recording errors
    NoActiveRecording,
    RecordingAlreadyActive,
    RecordingNotPaused,
    RecordingAlreadyPaused,
    RecordingFailed(String),
    
    // File system errors
    FileNotFound(String),
    PathTraversalAttempt,
    InvalidPath(String),
    PermissionDenied(String),
    IoError(String),
    
    // Hotkey errors
    HotkeyRegistrationFailed(String),
    HotkeyNotFound,
    InvalidHotkeyConfiguration,
    
    // Configuration errors
    ConfigurationError(String),
    InvalidConfiguration(String),
    
    // General errors
    InternalError(String),
    InvalidInput(String),
    NotImplemented(String),
}

impl fmt::Display for CapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Recording errors
            CapError::NoActiveRecording => write!(f, "No active recording"),
            CapError::RecordingAlreadyActive => write!(f, "Recording is already active"),
            CapError::RecordingNotPaused => write!(f, "Recording is not paused"),
            CapError::RecordingAlreadyPaused => write!(f, "Recording is already paused"),
            CapError::RecordingFailed(msg) => write!(f, "Recording failed: {}", msg),
            
            // File system errors
            CapError::FileNotFound(path) => write!(f, "File not found: {}", path),
            CapError::PathTraversalAttempt => write!(f, "Path traversal attempt detected"),
            CapError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            CapError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            CapError::IoError(msg) => write!(f, "IO error: {}", msg),
            
            // Hotkey errors
            CapError::HotkeyRegistrationFailed(msg) => write!(f, "Failed to register hotkey: {}", msg),
            CapError::HotkeyNotFound => write!(f, "Hotkey not found"),
            CapError::InvalidHotkeyConfiguration => write!(f, "Invalid hotkey configuration"),
            
            // Configuration errors
            CapError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            CapError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            
            // General errors
            CapError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            CapError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            CapError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
        }
    }
}

impl std::error::Error for CapError {}

// Conversion from std::io::Error
impl From<std::io::Error> for CapError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => CapError::FileNotFound(err.to_string()),
            std::io::ErrorKind::PermissionDenied => CapError::PermissionDenied(err.to_string()),
            _ => CapError::IoError(err.to_string()),
        }
    }
}

// For Tauri command compatibility - convert to String for IPC
impl From<CapError> for String {
    fn from(err: CapError) -> Self {
        err.to_string()
    }
}

pub type CapResult<T> = Result<T, CapError>;