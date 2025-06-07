# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust library crate that provides an asynchronous AWS SQS message receiver framework. The library abstracts SQS polling complexity and allows users to register custom message handlers with shared resources.

## Architecture

### Core Components

- **Client Module** (`src/client.rs`): SQS client factory functions supporting environment-based and explicit credential configuration
- **Receiver Module** (`src/receiver.rs`): Contains both functional API (`start_receive_queue`) and struct-based API (`AwsSqsReceiver`) for message processing
- **Handler Framework** (`src/receiver/functions.rs`): Trait-based system (`AsyncSqsReceiverFunction`) for defining message handlers with generic shared resource support
- **Error Types** (`src/errors.rs`): Custom error enum using `thiserror` for error handling

### Message Processing Flow

1. SQS client polls queues with long polling (20-second wait, up to 10 messages)
2. Messages are dispatched to registered handlers with shared resources
3. Successfully processed messages are automatically deleted from the queue
4. Errors are logged but processing continues for remaining messages

### API Patterns

The library supports two usage patterns:
- **Functional**: Direct use of `start_receive_queue()` with closure-based handlers
- **Object-oriented**: `AwsSqsReceiver` struct for managing multiple handlers via `add_handler()` and `start_all_handlers()`

## Development Commands

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run with cargo
cargo run

# Check code
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

## Dependencies

- **AWS SDK**: `aws-sdk-sqs` (1.71.0) and `aws-config` (1.1.7) for SQS operations
- **Async Runtime**: `tokio` (1.45.1) with full features for async processing
- **Error Handling**: `thiserror` (2.0.12) for ergonomic error types
- **Utilities**: `async-trait` (0.1.88) and `futures` (0.3.31) for async abstractions

## Key Design Considerations

- Handlers receive `(String, TShared)` parameters for message body and shared resources
- All handlers must return `Result<(), AwsSqsReceiverError>`
- Shared resources must implement `Send + Sync + Clone + 'static`
- The `start_all_handlers()` method consumes the receiver (takes ownership of `self`)
- Error handling uses continue-on-error semantics to maintain processing resilience