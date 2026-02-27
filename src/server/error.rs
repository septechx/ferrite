use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("No {0} loader version found for {1}")]
    LoaderVersionNotFound(String, String),

    #[error("No {0} version found")]
    VersionNotFound(String),

    #[error("No filename in Content-Disposition header")]
    MissingFilename,

    #[error("Failed to parse version format")]
    InvalidVersionFormat,

    #[error("Failed to parse XML response: {0}")]
    XmlParse(String),

    #[error("Failed to parse JSON response: {0}")]
    JsonParse(String),
}

impl From<serde_xml_rs::Error> for ServerError {
    fn from(e: serde_xml_rs::Error) -> Self {
        ServerError::XmlParse(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;
