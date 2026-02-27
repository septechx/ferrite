use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModError {
    #[error("A mod with ID or name '{0}' is not present in this profile")]
    NotFound(String),

    #[error("User cancelled selection")]
    Cancelled,

    #[error("Inquire error: {0}")]
    Inquire(String),
}

impl From<inquire::InquireError> for ModError {
    fn from(e: inquire::InquireError) -> Self {
        match e {
            inquire::InquireError::OperationCanceled
            | inquire::InquireError::OperationInterrupted => ModError::Cancelled,
            _ => ModError::Inquire(e.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, ModError>;
