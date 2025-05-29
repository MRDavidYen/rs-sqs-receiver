use crate::errors::AwsSqsReceiverError;

use std::marker::PhantomData;

pub struct SqsReceiverFunction<RFn, TShared>
where
    RFn: AsyncFn(String, TShared) -> Result<(), AwsSqsReceiverError> + Send + Sync,
    TShared: Send + Sync + Clone + 'static,
{
    rv_fn: RFn,
    _marker: PhantomData<TShared>,
}

impl<RFn, TShared> SqsReceiverFunction<RFn, TShared>
where
    RFn: AsyncFn(String, TShared) -> Result<(), AwsSqsReceiverError> + Send + Sync,
    TShared: Send + Sync + Clone + 'static,
{
    pub fn new(rv_fn: RFn) -> Self {
        SqsReceiverFunction {
            rv_fn,
            _marker: PhantomData,
        }
    }

    pub async fn start_receive(
        &self,
        sqs_client: aws_sdk_sqs::Client,
        queue_url: &str,
        shared_resources: TShared,
    ) {
        let sqs_client = sqs_client.clone();
        let queue_url = queue_url.to_string();

        tokio::spawn(async move {
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

                    if let Err(e) = (self.rv_fn)(message_body, shared_resources.clone()).await {
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
        });
    }

    /// Handles the incoming message by invoking the provided function with the message,
    pub async fn handle_message(
        &self,
        message: String,
        shared_resources: TShared,
    ) -> Result<(), AwsSqsReceiverError> {
        (self.rv_fn)(message, shared_resources).await
    }
}
