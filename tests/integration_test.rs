use rs_sqs_receiver::{client, errors::AwsSqsReceiverError, receiver};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

#[derive(Clone)]
struct SharedCounter {
    count: Arc<Mutex<i32>>,
}

impl SharedCounter {
    fn new() -> Self {
        Self {
            count: Arc::new(Mutex::new(0)),
        }
    }

    async fn increment(&self) {
        let mut count = self.count.lock().await;
        *count += 1;
    }

    async fn get_count(&self) -> i32 {
        *self.count.lock().await
    }
}

async fn test_handler(message: String, shared: SharedCounter) -> Result<(), AwsSqsReceiverError> {
    println!("Received message: {}", message);
    shared.increment().await;
    Ok(())
}

#[tokio::test]
async fn test_sqs_integration() {
    dotenvy::dotenv().ok();

    let queue_url = env::var("TEST_SQS_QUEUE_URL").expect("TEST_SQS_QUEUE_URL must be set");

    let sqs_client = client::create_sqs_client_from_env().await;

    sqs_client
        .send_message()
        .queue_url(&queue_url)
        .message_body("Test message 1")
        .message_deduplication_id("test-message-1")
        .message_group_id("test-group")
        .send()
        .await
        .expect("Failed to send test message 1");

    println!("Sent 2 test messages to queue");

    let shared_counter = SharedCounter::new();
    let counter_clone = shared_counter.clone();

    let client_clone = sqs_client.clone();
    let queue_url_clone = queue_url.clone();
    let receive_task = tokio::spawn(async move {
        receiver::start_receive_queue(client_clone, &queue_url_clone, counter_clone, test_handler)
            .await
    });

    tokio::time::sleep(Duration::from_secs(5)).await;

    let timeout_result = timeout(Duration::from_secs(30), async {
        loop {
            let count = shared_counter.get_count().await;
            println!("Current message count: {}", count);
            if count >= 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await;

    receive_task.abort();

    match timeout_result {
        Ok(_) => {
            let final_count = shared_counter.get_count().await;
            println!("Successfully processed {} messages", final_count);
            assert!(
                final_count >= 2,
                "Should have processed at least 2 messages"
            );
        }
        Err(_) => {
            let final_count = shared_counter.get_count().await;
            panic!("Test timed out. Only processed {} messages", final_count);
        }
    }
}

#[tokio::test]
async fn test_sqs_receiver_struct_api() {
    let queue_name = "test-queue-struct-api";

    let sqs_client = client::create_sqs_client_from_env().await;

    let queue_url = match sqs_client
        .create_queue()
        .queue_name(queue_name)
        .send()
        .await
    {
        Ok(output) => output.queue_url().unwrap().to_string(),
        Err(e) => {
            if e.to_string().contains("QueueAlreadyExists") {
                let queues = sqs_client
                    .list_queues()
                    .queue_name_prefix(queue_name)
                    .send()
                    .await
                    .expect("Failed to list queues");

                queues
                    .queue_urls()
                    .first()
                    .expect("Queue should exist but not found")
                    .clone()
            } else {
                panic!("Failed to create/get queue: {}", e);
            }
        }
    };

    sqs_client
        .send_message()
        .queue_url(&queue_url)
        .message_body("Struct API test message")
        .send()
        .await
        .expect("Failed to send test message");

    let shared_counter = SharedCounter::new();
    let counter_clone = shared_counter.clone();

    let mut receiver = receiver::AwsSqsReceiver::new();
    receiver.add_handler_fn(&queue_url, test_handler, counter_clone, None);

    let client_clone = sqs_client.clone();
    let receive_task = tokio::spawn(async move { receiver.start_all_handlers(client_clone).await });

    let timeout_result = timeout(Duration::from_secs(30), async {
        loop {
            let count = shared_counter.get_count().await;
            if count >= 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await;

    receive_task.abort();

    match timeout_result {
        Ok(_) => {
            let final_count = shared_counter.get_count().await;
            assert!(final_count >= 1, "Should have processed at least 1 message");
        }
        Err(_) => {
            let final_count = shared_counter.get_count().await;
            panic!(
                "Struct API test timed out. Only processed {} messages",
                final_count
            );
        }
    }

    let purge_result = sqs_client.purge_queue().queue_url(&queue_url).send().await;

    if let Err(e) = purge_result {
        println!("Warning: Failed to purge queue: {}", e);
    }

    println!("Struct API test completed successfully");
}

#[tokio::test]
async fn test_aws_sqs_receiver_comprehensive() {
    let queue_url_1 = env::var("TEST_SQS_QUEUE_URL").expect("TEST_SQS_QUEUE_URL must be set");

    let sqs_client = client::create_sqs_client_from_env().await;

    // Send messages to both queues
    sqs_client
        .send_message()
        .queue_url(&queue_url_1)
        .message_body("Message for queue 1 - handler with shared resource")
        .message_group_id("1")
        .message_deduplication_id("dedup-1")
        .send()
        .await
        .expect("Failed to send message to queue 1");

    sqs_client
        .send_message()
        .queue_url(&queue_url_1)
        .message_body("Another message for queue 1")
        .message_group_id("2")
        .message_deduplication_id("dedup-2")
        .send()
        .await
        .expect("Failed to send second message to queue 1");

    println!("Sent test messages to both queues");

    // Create shared resources
    let shared_counter = SharedCounter::new();
    let counter_clone = shared_counter.clone();

    // Create receiver and add multiple handlers
    let mut receiver = receiver::AwsSqsReceiver::new();

    // Add handler with shared resource for queue 1
    receiver.add_handler_fn(
        &queue_url_1,
        |message: String, shared: SharedCounter| async move {
            println!("Queue 1 handler received: {}", message);
            shared.increment().await;
            Ok(())
        },
        counter_clone,
        None,
    );

    // Add simple handler for queue 2
    // receiver.add_simple_handler(&queue_url_2, {
    //     let count_ref = simple_count_clone;
    //     move |message: String| {
    //         let count_ref = count_ref.clone();
    //         async move {
    //             println!("Queue 2 simple handler received: {}", message);
    //             let mut count = count_ref.lock().unwrap();
    //             *count += 1;
    //             Ok(())
    //         }
    //     }
    // });

    println!("Starting all handlers...");

    let client_clone = sqs_client.clone();

    receiver.start_all_handlers(client_clone).await;

    println!("Comprehensive AwsSqsReceiver test completed successfully");
}

#[tokio::test]
async fn test_aws_sqs_receiver_with_shutdown() {
    dotenvy::dotenv().ok();

    let queue_url = env::var("TEST_SQS_QUEUE_URL").expect("TEST_SQS_QUEUE_URL must be set");

    let sqs_client = client::create_sqs_client_from_env().await;

    // Send a test message
    sqs_client
        .send_message()
        .queue_url(&queue_url)
        .message_body("Shutdown test message")
        .message_deduplication_id("shutdown-test")
        .message_group_id("shutdown-test")
        .send()
        .await
        .expect("Failed to send test message");

    let shared_counter = SharedCounter::new();
    let counter_clone = shared_counter.clone();

    let mut receiver = receiver::AwsSqsReceiver::new();
    receiver.add_handler_fn(
        &queue_url,
        |message: String, shared: SharedCounter| async move {
            println!("Shutdown test handler received: {}", message);
            shared.increment().await;

            Ok(())
        },
        counter_clone,
        None,
    );

    // Create shutdown signal
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let client_clone = sqs_client.clone();
    let receive_task = tokio::spawn(async move {
        receiver
            .start_all_handlers_with_shutdown(client_clone, shutdown_rx)
            .await
    });

    // Wait a bit for message processing to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Send shutdown signal
    println!("Sending shutdown signal...");
    let _ = shutdown_tx.send(());

    // Wait for graceful shutdown
    let shutdown_result = timeout(Duration::from_secs(10), receive_task).await;

    match shutdown_result {
        Ok(task_result) => {
            if let Err(e) = task_result {
                println!("Task completed with error (expected for shutdown): {:?}", e);
            } else {
                println!("Handlers shut down gracefully");
            }
        }
        Err(_) => {
            panic!("Shutdown test timed out - handlers did not shut down gracefully");
        }
    }

    let final_count = shared_counter.get_count().await;
    println!("Messages processed before shutdown: {}", final_count);

    // We expect at least the one message we sent to be processed
    assert!(
        final_count >= 1,
        "Should have processed at least 1 message before shutdown"
    );

    let purge_result = sqs_client.purge_queue().queue_url(&queue_url).send().await;

    if let Err(e) = purge_result {
        println!("Warning: Failed to purge queue: {}", e);
    }

    println!("Shutdown test completed successfully");
}
