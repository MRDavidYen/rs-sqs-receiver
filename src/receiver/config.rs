/// Configuration for the AWS SQS receiver.
///
/// This struct defines the parameters for receiving messages from an SQS queue,
/// including the maximum number of messages to receive and the wait time for long polling.
///
/// # Fields
/// - `max_number_of_messages`: The maximum number of messages to receive in a single request.
/// - `wait_time_seconds`: The wait time for long polling, in seconds.
#[derive(Debug, Clone)]
pub struct AwsSqsReceiverConfig {
    /// The maximum number of messages to receive in a single request.
    pub max_number_of_messages: i32,

    /// The wait time for long polling, in seconds.
    pub wait_time_seconds: i32,
}

impl Default for AwsSqsReceiverConfig {
    fn default() -> Self {
        AwsSqsReceiverConfig {
            max_number_of_messages: 10,
            wait_time_seconds: 20,
        }
    }
}
