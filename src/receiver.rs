use aws_config::Region;
use aws_sdk_sqs::config::SharedCredentialsProvider;

mod functions;

/// A struct that wraps the AWS SQS client.
pub struct AwsSqsReceiver<T>
where
    T: Send + Sync,
{
    /// The AWS SQS client used to interact with the SQS service.
    sqs_client: aws_sdk_sqs::Client,

    /// A shared resource that can be used across multiple threads.
    shared_resources: T,
}

impl<T> AwsSqsReceiver<T>
where
    T: Send + Sync,
{
    pub fn new(sqs_client: aws_sdk_sqs::Client, shared_resources: T) -> Self {
        AwsSqsReceiver {
            sqs_client,
            shared_resources,
        }
    }
}
