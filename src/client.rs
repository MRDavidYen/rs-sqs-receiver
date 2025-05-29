use aws_config::Region;
use aws_sdk_sqs::config::SharedCredentialsProvider;

pub async fn create_sqs_client_with_env() -> aws_sdk_sqs::Client {
    let config = aws_config::load_from_env().await;
    let sqs_client = aws_sdk_sqs::Client::new(&config);

    sqs_client
}

/// Creates a new instance of `sqs_client` with specified AWS credentials and region.
pub fn create_sqs_client_with_credentials(
    access_key_id: &str,
    secret_access_key: &str,
    region: &str,
) -> aws_sdk_sqs::Client {
    let credentials =
        aws_sdk_sqs::config::Credentials::new(access_key_id, secret_access_key, None, None, "aws");

    let shared_credentials = SharedCredentialsProvider::new(credentials);

    let config = aws_sdk_sqs::config::Builder::new()
        .region(Region::new(region.to_string()))
        .credentials_provider(shared_credentials)
        .build();

    aws_sdk_sqs::Client::from_conf(config)
}
