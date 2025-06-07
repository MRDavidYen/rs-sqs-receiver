use std::str::FromStr;

use thiserror::Error;

/// Error types for AWS SQS receiver operations.
///
/// This enum represents all possible errors that can occur during
/// SQS message receiving and processing operations.
#[derive(Debug, Error)]
pub enum AwsSqsReceiverError {
    /// Error that occurs during AWS SQS client initialization.
    ///
    /// This error typically happens when there are issues with AWS credentials,
    /// region configuration, or network connectivity during client setup.
    #[error("failed to initialize AWS SQS client: {0}")]
    InitializationError(String),

    #[error("{0}")]
    GenericError(#[from] GenericError),
}

/// Generic error type for handling unexpected errors.
#[derive(Debug, Error)]
pub struct GenericError(String);

impl GenericError {
    /// Creates a new `GenericError` with the provided message.
    pub fn new(message: String) -> Self {
        GenericError(message)
    }
}

impl std::fmt::Display for GenericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for GenericError {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(GenericError::new(s.to_string()))
    }
}

impl From<String> for GenericError {
    fn from(s: String) -> Self {
        GenericError::new(s)
    }
}
