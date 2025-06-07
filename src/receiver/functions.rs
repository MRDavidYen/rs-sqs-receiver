use crate::errors::AwsSqsReceiverError;
use async_trait::async_trait;
use std::future::Future;

/// Trait for implementing asynchronous SQS message receivers.
///
/// This trait provides a common interface for starting SQS message reception
/// and processing. Implementations handle the details of polling, message
/// processing, and error handling.
#[async_trait]
pub trait AsyncSqsReceiverFunction: Send + Sync {
    /// Starts the SQS message receiving process.
    ///
    /// # Arguments
    ///
    /// * `sqs_client` - The AWS SQS client to use for message operations
    async fn start_receive(&self, sqs_client: aws_sdk_sqs::Client);
}

#[async_trait]
impl<F, Fut, TShared> AsyncSqsReceiverFunction for AsyncSqsReceiverFunctionImpl<F, Fut, TShared>
where
    F: Fn(String, TShared) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = Result<(), AwsSqsReceiverError>> + Send + 'static,
    TShared: Send + Sync + Clone + 'static,
{
    async fn start_receive(&self, sqs_client: aws_sdk_sqs::Client) {
        let resource = self.shared_resources.clone();
        let queue_url = self.queue_url.clone();
        let rv_fn = self.rv_fn.clone();

        loop {
            let receive_message = sqs_client
                .receive_message()
                .queue_url(&queue_url)
                .max_number_of_messages(10)
                .wait_time_seconds(20)
                .send()
                .await;
            
            if receive_message.is_err() {
                eprintln!("Error receiving messages: {:?}", receive_message.err());
                continue;
            }
            let receive_message = receive_message.unwrap();
            let messages = receive_message.messages();
            if messages.is_empty() {
                println!("No messages received.");
                continue;
            }
            for message in messages {
                let message_body = message.body();
                if message_body.is_none() {
                    eprintln!("Received a message with no body.");
                    continue;
                }
                let message_body = message_body.unwrap().to_string();
                println!("Received message: {}", message_body);
                if let Err(e) = (rv_fn)(message_body, resource.clone()).await {
                    eprintln!("Error handling message: {:?}", e);
                } else {
                    // Delete the message after successful processing
                    if let Err(delete_err) = sqs_client
                        .delete_message()
                        .queue_url(&queue_url)
                        .receipt_handle(message.receipt_handle().unwrap())
                        .send()
                        .await
                    {
                        eprintln!("Error deleting message: {:?}", delete_err);
                    }
                }
            }
        }
    }
}

/// Implementation of `AsyncSqsReceiverFunction` that handles SQS message processing.
///
/// This struct wraps a user-provided function and shared resources, providing
/// the logic for polling SQS queues, processing messages, and handling errors.
///
/// # Type Parameters
///
/// * `RFn` - The message handler function type
/// * `Fut` - The future returned by the handler function
/// * `TShared` - The type of shared resources passed to the handler
pub struct AsyncSqsReceiverFunctionImpl<RFn, Fut, TShared>
where
    RFn: Fn(String, TShared) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = Result<(), AwsSqsReceiverError>> + Send + 'static,
    TShared: Send + Sync + Clone + 'static,
{
    rv_fn: RFn,
    queue_url: String,
    shared_resources: TShared,
}

impl<RFn, Fut, TShared> AsyncSqsReceiverFunctionImpl<RFn, Fut, TShared>
where
    RFn: Fn(String, TShared) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = Result<(), AwsSqsReceiverError>> + Send + 'static,
    TShared: Send + Sync + Clone + 'static,
{
    /// Creates a new instance of the SQS receiver function implementation.
    ///
    /// # Arguments
    ///
    /// * `rv_fn` - The message handler function
    /// * `queue_url` - The SQS queue URL to poll
    /// * `shared_resources` - Resources shared between message processing calls
    ///
    /// # Returns
    ///
    /// Returns a new `AsyncSqsReceiverFunctionImpl` instance.
    pub fn new(rv_fn: RFn, queue_url: &str, shared_resources: TShared) -> Self {
        AsyncSqsReceiverFunctionImpl {
            rv_fn,
            queue_url: queue_url.to_string(),
            shared_resources,
        }
    }
}
