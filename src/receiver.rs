use std::future::Future;

use functions::AsyncSqsReceiverFunction;

use crate::errors::AwsSqsReceiverError;

mod config;
pub use config::AwsSqsReceiverConfig;

mod functions;

/// Starts receiving messages from an SQS queue using the provided function.
///
/// This is the functional API for processing SQS messages. It spawns a background task
/// that continuously polls the specified SQS queue for messages, processes them using
/// the provided handler function, and automatically deletes successfully processed messages.
///
/// # Arguments
///
/// * `sqs_client` - The AWS SQS client to use for queue operations
/// * `queue_url` - The URL of the SQS queue to poll
/// * `rv_fn` - The message handler function that processes each message
/// * `shared_resources` - Resources shared across all message processing calls
///
/// # Type Parameters
///
/// * `Rfn` - The message handler function type
/// * `TShared` - The type of shared resources
/// * `Fut` - The future returned by the handler function
///
/// # Example
///
/// ```rust,no_run
/// use aws_sqs_receiver::{client::create_sqs_client_from_env, receiver::start_receive_queue};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = create_sqs_client_from_env().await;
///     let queue_url = "https://sqs.region.amazonaws.com/account/queue-name";
///     let database_pool = "shared_db_connection".to_string();
///
///     start_receive_queue(
///         client,
///         queue_url,
///         database_pool,
///         |message, db_pool| async move {
///             println!("Processing {} with DB: {}", message, db_pool);
///             Ok(())
///         }
///     ).await;
///
///     Ok(())
/// }
/// ```
pub async fn start_receive_queue<Rfn, TShared, Fut>(
    sqs_client: aws_sdk_sqs::Client,
    queue_url: &str,
    shared_resources: TShared,
    rv_fn: Rfn,
) where
    Rfn: Fn(String, TShared) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), AwsSqsReceiverError>> + Send + 'static,
    TShared: Send + Sync + Clone + 'static,
{
    let queue_url = queue_url.to_string();

    // spawn a new task to loop and receive messages from SQS
    let task = tokio::spawn(async move {
        loop {
            let receive_message = sqs_client
                .receive_message()
                .queue_url(&queue_url)
                .max_number_of_messages(10)
                .wait_time_seconds(20)
                .send()
                .await;

            if let Err(e) = receive_message {
                eprintln!("Error receiving messages: {:?}", e);
                continue;
            }

            let receive_message = receive_message.unwrap();
            let messages = receive_message.messages();

            if messages.is_empty() {
                println!("No messages received.");
                continue;
            }

            for message in messages {
                let message_body = message.body().unwrap_or_default().to_string();
                println!("Received message: {}", message_body);

                if let Err(e) = (rv_fn)(message_body, shared_resources.clone()).await {
                    eprintln!("Error processing message: {:?}", e);
                }

                // delete the message after processing

                let msg_receipt_handle = message.receipt_handle();

                if msg_receipt_handle.is_none() {
                    eprintln!("Received a message with no receipt handle.");

                    continue;
                }

                let msg_receipt_handle = msg_receipt_handle.unwrap();

                if let Err(e) = sqs_client
                    .delete_message()
                    .queue_url(&queue_url)
                    .receipt_handle(msg_receipt_handle)
                    .send()
                    .await
                {
                    eprintln!("Error deleting message: {:?}", e);
                } else {
                    println!("Message deleted successfully.");
                }
            }
        }
    });

    // Wait for the task to complete (which should never happen in normal operation)
    match task.await {
        Err(e) => {
            eprintln!("Handler task panicked: {:?}", e);
        }
        _ => (),
    }
}

/// A struct-based AWS SQS receiver that supports multiple message handlers.
///
/// This provides an object-oriented API for managing multiple SQS queue handlers.
/// You can register handlers for different queues and start them all concurrently
/// using the `start_all_handlers` method.
///
/// # Example
///
/// ```rust,no_run
/// use aws_sqs_receiver::{client::create_sqs_client_from_env, receiver::AwsSqsReceiver};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = create_sqs_client_from_env().await;
///     let mut receiver = AwsSqsReceiver::new();
///
///     // Add multiple handlers
///     receiver.add_simple_handler(
///         "https://sqs.region.amazonaws.com/account/queue1",
///         |message| async move {
///             println!("Queue 1: {}", message);
///             Ok(())
///         }
///     );
///
///     receiver.add_simple_handler(
///         "https://sqs.region.amazonaws.com/account/queue2",
///         |message| async move {
///             println!("Queue 2: {}", message);
///             Ok(())
///         }
///     );
///
///     // Start all handlers (this consumes the receiver)
///     receiver.start_all_handlers(client).await;
///
///     Ok(())
/// }
/// ```
pub struct AwsSqsReceiver {
    handlers: Vec<Box<dyn AsyncSqsReceiverFunction>>,
}

impl Default for AwsSqsReceiver {
    fn default() -> Self {
        Self::new()
    }
}

impl AwsSqsReceiver {
    /// Creates a new empty `AwsSqsReceiver` instance.
    ///
    /// # Returns
    ///
    /// A new `AwsSqsReceiver` with no registered handlers.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Adds a boxed handler to the receiver.
    ///
    /// This is the low-level method for adding handlers. Most users should prefer
    /// the convenience methods `add_handler_fn` or `add_simple_handler`.
    ///
    /// # Arguments
    ///
    /// * `handler` - A boxed trait object implementing `AsyncSqsReceiverFunction`
    pub fn add_handler(&mut self, handler: Box<dyn AsyncSqsReceiverFunction>) {
        self.handlers.push(handler);
    }

    /// Convenience method to add a handler with a closure.
    ///
    /// # Example
    /// ```
    /// use aws_sqs_receiver::receiver::AwsSqsReceiver;
    ///
    /// let mut receiver = AwsSqsReceiver::new();
    /// let shared_resource = "shared_data".to_string();
    ///
    /// receiver.add_handler_fn(
    ///     "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue",
    ///     |message: String, shared: String| async move {
    ///         // Process message
    ///         println!("Received: {} with shared: {}", message, shared);
    ///         Ok(())
    ///     },
    ///     shared_resource
    /// );
    /// ```
    pub fn add_handler_fn<Rfn, TShared, Fut>(
        &mut self,
        queue_url: &str,
        handler_fn: Rfn,
        shared_resources: TShared,
        config: Option<AwsSqsReceiverConfig>,
    ) where
        Rfn: Fn(String, TShared) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<(), crate::errors::AwsSqsReceiverError>> + Send + 'static,
        TShared: Send + Sync + Clone + 'static,
    {
        let mut handler =
            functions::AsyncSqsReceiverFunctionImpl::new(handler_fn, queue_url, shared_resources);

        // If a configuration is provided, apply it to the handler
        if let Some(cfg) = config {
            handler = handler.with_config(cfg);
        }

        self.handlers.push(Box::new(handler));
    }

    /// Convenience method to add a handler that takes only the message (no shared resources).
    ///
    /// # Example
    /// ```
    /// use aws_sqs_receiver::receiver::AwsSqsReceiver;
    ///
    /// let mut receiver = AwsSqsReceiver::new();
    /// receiver.add_simple_handler(
    ///     "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue",
    ///     |message: String| async move {
    ///         println!("Received: {}", message);
    ///         Ok(())
    ///     }
    /// );
    /// ```
    pub fn add_simple_handler<Rfn, Fut>(
        &mut self,
        queue_url: &str,
        handler_fn: Rfn,
        config: Option<AwsSqsReceiverConfig>,
    ) where
        Rfn: Fn(String) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Result<(), crate::errors::AwsSqsReceiverError>> + Send + 'static,
    {
        // Wrap the single-argument closure to match the two-argument trait signature
        let wrapped_fn = move |message: String, _: ()| (handler_fn)(message);
        self.add_handler_fn(queue_url, wrapped_fn, (), config);
    }

    /// Starts all handlers to receive messages from SQS queues.
    /// This method blocks indefinitely until all handlers complete (which should never happen in normal operation).
    /// For long-running applications, this is the main blocking call that keeps the application alive.
    pub async fn start_all_handlers(self, sqs_client: aws_sdk_sqs::Client) {
        let mut tasks = Vec::new();

        for handler in self.handlers.into_iter() {
            let sqs_client = sqs_client.clone();

            let task = tokio::spawn(async move {
                handler.start_receive(sqs_client).await;
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete concurrently (which should never happen in normal operation)
        // If a task panics, log the error but continue waiting for other tasks
        let results = futures::future::join_all(tasks).await;

        for result in results {
            if let Err(e) = result {
                eprintln!("Handler task panicked: {:?}", e);
            }
        }
    }

    /// Starts all handlers with graceful shutdown support using a cancellation token.
    /// This method blocks until either all handlers complete or the shutdown signal is received.
    pub async fn start_all_handlers_with_shutdown(
        self,
        sqs_client: aws_sdk_sqs::Client,
        mut shutdown_signal: tokio::sync::oneshot::Receiver<()>,
    ) {
        let mut tasks = Vec::new();

        for handler in self.handlers.into_iter() {
            let sqs_client = sqs_client.clone();

            let task = tokio::spawn(async move {
                handler.start_receive(sqs_client).await;
            });

            tasks.push(task);
        }

        // Create abort handles for shutdown
        let abort_handles: Vec<_> = tasks.iter().map(|task| task.abort_handle()).collect();

        // Wait for either shutdown signal or all tasks to complete
        tokio::select! {
            _ = &mut shutdown_signal => {
                println!("Shutdown signal received, stopping handlers...");

                // Abort all running tasks
                for abort_handle in abort_handles {
                    abort_handle.abort();
                }
            }
            _ = async {
                // Wait for all tasks to complete concurrently (which should never happen in normal operation)
                let results = futures::future::join_all(tasks).await;

                for result in results {
                    if let Err(e) = result {
                        eprintln!("Handler task panicked: {:?}", e);
                    }
                }
            } => {
                println!("All handlers completed unexpectedly");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receiver::functions::AsyncSqsReceiverFunctionImpl;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[derive(Clone)]
    struct TestSharedResource {
        messages: Arc<Mutex<Vec<String>>>,
    }

    #[tokio::test]
    async fn test_aws_sqs_receiver_new() {
        let receiver = AwsSqsReceiver::new();
        assert_eq!(receiver.handlers.len(), 0);
    }

    #[tokio::test]
    async fn test_aws_sqs_receiver_add_handler() {
        let mut receiver = AwsSqsReceiver::new();

        let shared_resource = TestSharedResource {
            messages: Arc::new(Mutex::new(Vec::new())),
        };

        let handler = AsyncSqsReceiverFunctionImpl::new(
            |message: String, resource: TestSharedResource| async move {
                resource.messages.lock().unwrap().push(message);
                Ok(())
            },
            "test-queue-url",
            shared_resource,
        );

        receiver.add_handler(Box::new(handler));
        assert_eq!(receiver.handlers.len(), 1);
    }

    #[tokio::test]
    async fn test_aws_sqs_receiver_add_handler_fn() {
        let mut receiver = AwsSqsReceiver::new();

        let shared_resource = TestSharedResource {
            messages: Arc::new(Mutex::new(Vec::new())),
        };

        // Test the convenience method
        receiver.add_handler_fn(
            "test-queue-url",
            |message: String, resource: TestSharedResource| async move {
                resource.messages.lock().unwrap().push(message);
                Ok(())
            },
            shared_resource,
            None,
        );

        assert_eq!(receiver.handlers.len(), 1);
    }

    #[tokio::test]
    async fn test_aws_sqs_receiver_add_simple_handler() {
        let mut receiver = AwsSqsReceiver::new();

        // Test the simple handler convenience method
        receiver.add_simple_handler("test-queue-url", |message: String| async move {
            println!("Processing message: {}", message);
            Ok(())
        }, None);

        assert_eq!(receiver.handlers.len(), 1);
    }

    #[tokio::test]
    async fn test_aws_sqs_receiver_multiple_handlers() {
        let mut receiver = AwsSqsReceiver::new();

        let shared_resource1 = TestSharedResource {
            messages: Arc::new(Mutex::new(Vec::new())),
        };
        let shared_resource2 = TestSharedResource {
            messages: Arc::new(Mutex::new(Vec::new())),
        };

        // Mix of convenience methods and original method
        receiver.add_handler_fn(
            "test-queue-1",
            |message: String, resource: TestSharedResource| async move {
                resource
                    .messages
                    .lock()
                    .unwrap()
                    .push(format!("handler1: {}", message));
                Ok(())
            },
            shared_resource1,
            None,
        );

        receiver.add_handler_fn(
            "test-queue-2",
            |message: String, resource: TestSharedResource| async move {
                resource
                    .messages
                    .lock()
                    .unwrap()
                    .push(format!("handler2: {}", message));
                Ok(())
            },
            shared_resource2,
            None,
        );

        // Add a simple handler too
        receiver.add_simple_handler("test-queue-3", |message: String| async move {
            println!("Simple handler got: {}", message);
            Ok(())
        }, None);

        assert_eq!(receiver.handlers.len(), 3);
    }

    #[tokio::test]
    async fn test_start_all_handlers_ownership() {
        let mut receiver = AwsSqsReceiver::new();

        // Use the convenience method instead of manual boxing
        receiver.add_simple_handler("test-queue", |_message: String| async move {
            // Mock handler that does nothing for this test
            tokio::time::sleep(Duration::from_millis(1)).await;
            Ok(())
        }, None);

        assert_eq!(receiver.handlers.len(), 1);

        // Create a mock SQS client config for testing
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region("us-east-1")
            .load()
            .await;
        let sqs_client = aws_sdk_sqs::Client::new(&config);

        // Test that start_all_handlers consumes the receiver
        // Spawn this in a task with timeout to avoid blocking indefinitely
        let task = tokio::spawn(async move {
            receiver.start_all_handlers(sqs_client).await;
        });

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Abort the task since it would run indefinitely
        task.abort();

        // The key test is that the receiver was moved and cannot be used again
        // This compilation test verifies the ownership semantics work correctly
    }

    #[tokio::test]
    async fn test_start_all_handlers_with_shutdown() {
        let mut receiver = AwsSqsReceiver::new();

        let shared_resource = TestSharedResource {
            messages: Arc::new(Mutex::new(Vec::new())),
        };

        let handler = AsyncSqsReceiverFunctionImpl::new(
            |_message: String, _resource: TestSharedResource| async move {
                // Mock handler that simulates processing
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(())
            },
            "test-queue",
            shared_resource,
        );

        receiver.add_handler(Box::new(handler));

        // Create a mock SQS client config for testing
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region("us-east-1")
            .load()
            .await;
        let sqs_client = aws_sdk_sqs::Client::new(&config);

        // Create shutdown signal
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Start handlers with shutdown signal
        let handler_task = tokio::spawn(async move {
            receiver
                .start_all_handlers_with_shutdown(sqs_client, shutdown_rx)
                .await;
        });

        // Give handlers a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Send shutdown signal - ignore result since receiver might be dropped if handlers completed
        let _ = shutdown_tx.send(());

        // Wait for handlers to shut down
        let result = tokio::time::timeout(Duration::from_millis(500), handler_task).await;
        assert!(
            result.is_ok(),
            "Handlers should shut down or complete within timeout"
        );
    }
}
