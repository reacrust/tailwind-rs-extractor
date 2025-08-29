use thiserror::Error;

/// Main error type for the tailwind-extractor crate
#[derive(Debug, Error)]
pub enum ExtractorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Pattern error: {0}")]
    Pattern(#[from] glob::PatternError),

    #[error("Glob error: {0}")]
    Glob(#[from] glob::GlobError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("No files found matching the provided patterns")]
    NoFilesFound,

    #[error("Failed to parse file {path}: {message}")]
    ParseError { path: String, message: String },

    #[error("Failed to write output to {path}: {message}")]
    OutputError { path: String, message: String },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    #[error("Tailwind processing error: {0}")]
    TailwindError(String),

    #[error("Obfuscation error: {0}")]
    ObfuscationError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Input error: {0}")]
    InputError(String),
    
    #[error("Security violation: {0}")]
    SecurityError(String),
    
    #[error("Performance limit exceeded: {0}")]
    PerformanceError(String),
}

pub type Result<T> = std::result::Result<T, ExtractorError>;