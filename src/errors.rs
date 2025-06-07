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
}
