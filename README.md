# rs-sqs-receiver

[![Crates.io](https://img.shields.io/crates/v/rs-sqs-receiver.svg)](https://crates.io/crates/rs-sqs-receiver)
[![Documentation](https://docs.rs/rs-sqs-receiver/badge.svg)](https://docs.rs/rs-sqs-receiver)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

An asynchronous AWS SQS message receiver framework for Rust that abstracts SQS polling complexity and allows you to register custom message handlers with shared resources.

## Features

- 🚀 **Asynchronous message processing** with Tokio
- 🔧 **Trait-based handler system** with generic shared resource support
- 📦 **Two API patterns**: functional and object-oriented
- ✅ **Automatic message deletion** on successful processing
- 🛡️ **Continue-on-error semantics** for resilient processing
- ⏱️ **Long polling** with configurable parameters (20-second wait, up to 10 messages)
- 🔄 **Graceful shutdown** support with cancellation tokens

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rs-sqs-receiver = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

### Functional API

The simplest way to start receiving messages:

```rust
use rs_sqs_receiver::{client::create_sqs_client_from_env, receiver::start_receive_queue};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_sqs_client_from_env().await;
    let queue_url = "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue";
    let database_pool = "shared_db_connection".to_string();

    start_receive_queue(
        client,
        queue_url,
        database_pool,
        |message, db_pool| async move {
            println!("Processing message: {} with DB: {}", message, db_pool);
            // Your message processing logic here
            Ok(())
        }
    ).await;

    Ok(())
}
```

### Object-Oriented API

For more complex scenarios with multiple queues:

```rust
use rs_sqs_receiver::{client::create_sqs_client_from_env, receiver::AwsSqsReceiver};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = create_sqs_client_from_env().await;
    let mut receiver = AwsSqsReceiver::new();

    // Add handler with shared resources
    let database_pool = "db_connection".to_string();
    receiver.add_handler_fn(
        "https://sqs.us-east-1.amazonaws.com/123456789012/orders-queue",
        |message: String, db: String| async move {
            println!("Processing order: {} with DB: {}", message, db);
            Ok(())
        },
        database_pool,
        None, // Optional config parameter
    );

    // Add simple handler (no shared resources)
    receiver.add_simple_handler(
        "https://sqs.us-east-1.amazonaws.com/123456789012/notifications-queue",
        |message: String| async move {
            println!("Sending notification: {}", message);
            Ok(())
        },
        None, // Optional config parameter
    );

    // Start all handlers (this consumes the receiver)
    receiver.start_all_handlers(client).await;

    Ok(())
}
```

## AWS Configuration

### Environment Variables

Set up your AWS credentials using environment variables:

```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=us-east-1
```

### Explicit Credentials

```rust
use rs_sqs_receiver::client::create_sqs_client_with_credentials;

let client = create_sqs_client_with_credentials(
    "your_access_key",
    "your_secret_key",
    "us-east-1"
);
```

## Advanced Usage

### Configuration Options

You can customize SQS polling behavior using `AwsSqsReceiverConfig`:

```rust
use rs_sqs_receiver::{
    client::create_sqs_client_from_env, 
    receiver::{AwsSqsReceiver, AwsSqsReceiverConfig}
};

let mut receiver = AwsSqsReceiver::new();

// Create custom configuration
let config = AwsSqsReceiverConfig {
    max_number_of_messages: 5,  // Receive up to 5 messages per poll (default: 10)
    wait_time_seconds: 15,      // Wait 15 seconds for messages (default: 20)
};

// Use config with handler
receiver.add_handler_fn(
    "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue",
    |message: String, _: ()| async move {
        println!("Processing: {}", message);
        Ok(())
    },
    (),
    Some(config), // Pass the config
);

// Or use default configuration by passing None
receiver.add_simple_handler(
    "https://sqs.us-east-1.amazonaws.com/123456789012/other-queue",
    |message: String| async move {
        println!("Processing: {}", message);
        Ok(())
    },
    None, // Use default config (10 messages, 20 second wait)
);
```

### Shared Resources

You can share any type that implements `Send + Sync + Clone + 'static`:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    db_pool: Arc<DatabasePool>,
    cache: Arc<Mutex<HashMap<String, String>>>,
    config: AppConfig,
}

let shared_state = AppState {
    db_pool: Arc::new(create_db_pool()),
    cache: Arc::new(Mutex::new(HashMap::new())),
    config: load_config(),
};

receiver.add_handler_fn(
    queue_url,
    |message: String, state: AppState| async move {
        // Use state.db_pool, state.cache, etc.
        Ok(())
    },
    shared_state,
    None, // Optional config parameter
);
```

### Error Handling

Handlers must return `Result<(), AwsSqsReceiverError>`:

```rust
use rs_sqs_receiver::errors::AwsSqsReceiverError;

receiver.add_handler_fn(
    queue_url,
    |message: String, _shared: ()| async move {
        match process_message(&message).await {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Failed to process message: {}", e);
                Err(AwsSqsReceiverError::ProcessingError(e.to_string()))
            }
        }
    },
    (),
    None, // Optional config parameter
);
```

### Graceful Shutdown

Use the shutdown-aware API for graceful application termination:

```rust
use tokio::sync::oneshot;

let mut receiver = AwsSqsReceiver::new();
// ... add handlers ...

let (shutdown_tx, shutdown_rx) = oneshot::channel();

// Start handlers with shutdown support
let handler_task = tokio::spawn(async move {
    receiver.start_all_handlers_with_shutdown(client, shutdown_rx).await;
});

// Later, trigger shutdown
shutdown_tx.send(()).expect("Failed to send shutdown signal");

// Wait for graceful shutdown
handler_task.await.expect("Handler task failed");
```

## Message Processing Flow

1. **Long Polling**: The client polls SQS queues with 20-second wait times, requesting up to 10 messages per poll
2. **Message Dispatch**: Each received message is dispatched to the registered handler with shared resources
3. **Processing**: Your handler function processes the message
4. **Automatic Deletion**: Successfully processed messages are automatically deleted from the queue
5. **Error Handling**: Failed messages remain in the queue and processing continues for other messages

## API Reference

### Core Functions

- `start_receive_queue()`: Functional API for single queue processing
- `AwsSqsReceiver::new()`: Create a new receiver instance
- `add_handler_fn(queue_url, handler_fn, shared_resources, config)`: Add handler with shared resources and optional config
- `add_simple_handler(queue_url, handler_fn, config)`: Add handler without shared resources, with optional config
- `start_all_handlers()`: Start all registered handlers
- `start_all_handlers_with_shutdown()`: Start with graceful shutdown support

### Configuration

- `AwsSqsReceiverConfig`: Configuration struct for customizing SQS polling behavior
  - `max_number_of_messages`: Maximum messages per poll (1-10, default: 10)
  - `wait_time_seconds`: Long polling wait time in seconds (0-20, default: 20)

### Client Creation

- `create_sqs_client_from_env()`: Create client from environment variables
- `create_sqs_client_with_credentials()`: Create client with explicit credentials

## Development

### Prerequisites

- Rust 2024 edition
- AWS account with SQS access
- AWS credentials configured

### Commands

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run integration tests (requires AWS credentials)
TEST_SQS_QUEUE_URL=https://sqs.us-east-1.amazonaws.com/123456789012/test-queue cargo test

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

### Testing

Integration tests require AWS credentials and a test SQS queue. Set the `TEST_SQS_QUEUE_URL` environment variable to your test queue URL.

## Dependencies

- **AWS SDK**: `aws-sdk-sqs` and `aws-config` for SQS operations
- **Async Runtime**: `tokio` with full features
- **Error Handling**: `thiserror` for ergonomic error types
- **Async Utilities**: `async-trait` and `futures` for async abstractions

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Changelog

### 0.1.0

- Initial release
- Functional and object-oriented APIs
- Shared resource support
- Graceful shutdown capabilities
- Comprehensive error handling