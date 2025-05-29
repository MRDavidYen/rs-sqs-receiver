use thiserror::Error;

#[derive(Debug, Error)]
pub enum AwsSqsReceiverError {
    #[error("failed to initialize AWS SQS client: {0}")]
    InitializationError(String),
}
