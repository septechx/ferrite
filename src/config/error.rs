use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse config: {0}")]
    Parse(String),

    #[error("Failed to run pass command: {0}")]
    PassCommand(String),

    #[error("Failed to get current directory")]
    CurrentDirectory,

    #[error("Failed to serialize config: {0}")]
    Serialize(String),
}

impl From<serde_norway::Error> for ConfigError {
    fn from(e: serde_norway::Error) -> Self {
        ConfigError::Parse(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ConfigError>;
