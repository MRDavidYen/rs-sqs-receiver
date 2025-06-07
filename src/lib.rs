//! # AWS SQS Receiver
//!
//! An asynchronous AWS SQS message receiver framework that abstracts SQS polling complexity
//! and allows users to register custom message handlers with shared resources.
//!
//! ## Features
//!
//! - Asynchronous SQS message processing with tokio
//! - Trait-based handler system with generic shared resource support
//! - Both functional and object-oriented API patterns
//! - Automatic message deletion on successful processing
//! - Continue-on-error semantics for resilient processing
//! - Long polling with configurable parameters
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use aws_sqs_receiver::{client::create_sqs_client_from_env, receiver::start_receive_queue};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = create_sqs_client_from_env().await;
//!     let queue_url = "https://sqs.region.amazonaws.com/account/queue-name";
//!     let shared_data = "shared state".to_string();
//!
//!     start_receive_queue(
//!         client,
//!         queue_url,
//!         shared_data,
//!         |message, shared| async move {
//!             println!("Processing message: {} with shared: {}", message, shared);
//!             Ok(())
//!         }
//!     ).await;
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod errors;
pub mod receiver;
