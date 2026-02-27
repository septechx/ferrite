use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpgradeError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to run threads to completion")]
    ThreadJoin,

    #[error("Progress bar template parse failure")]
    ProgressBarTemplate,

    #[error("Mod '{0}' has no slug")]
    MissingSlug(String),

    #[error("File copy error: {0}")]
    FileCopy(String),

    #[error("Could not determine whether installable is a file or folder")]
    InvalidInstallable,

    #[error("Channel send error")]
    ChannelSend,

    #[error("Download error: {0}")]
    Download(String),
}

impl<T> From<std::sync::mpsc::SendError<T>> for UpgradeError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        UpgradeError::ChannelSend
    }
}

impl From<fs_extra::error::Error> for UpgradeError {
    fn from(e: fs_extra::error::Error) -> Self {
        UpgradeError::FileCopy(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, UpgradeError>;
