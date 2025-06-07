use aws_config::Region;
use aws_sdk_sqs::config::SharedCredentialsProvider;

/// Creates an AWS SQS client using credentials and configuration from the environment.
///
/// This function loads AWS configuration from environment variables such as:
/// - `AWS_ACCESS_KEY_ID`
/// - `AWS_SECRET_ACCESS_KEY`
/// - `AWS_REGION`
/// - `AWS_PROFILE`
///
/// # Returns
///
/// Returns a configured `aws_sdk_sqs::Client` ready for use.
///
/// # Example
///
/// ```rust,no_run
/// use aws_sqs_receiver::client::create_sqs_client_from_env;
///
/// #[tokio::main]
/// async fn main() {
///     let client = create_sqs_client_from_env().await;
///     // Use the client...
/// }
/// ```
pub async fn create_sqs_client_from_env() -> aws_sdk_sqs::Client {
    let config = aws_config::load_from_env().await;
    aws_sdk_sqs::Client::new(&config)
}

/// Creates an AWS SQS client with explicitly provided credentials and region.
///
/// This function creates a client with specific AWS credentials rather than
/// loading them from the environment. Useful for applications that manage
/// credentials dynamically or need to use different credentials than those
/// in the environment.
///
/// # Arguments
///
/// * `access_key_id` - The AWS access key ID
/// * `secret_access_key` - The AWS secret access key
/// * `region` - The AWS region (e.g., "us-east-1", "eu-west-1")
///
/// # Returns
///
/// Returns a configured `aws_sdk_sqs::Client` ready for use.
///
/// # Example
///
/// ```rust,no_run
/// use aws_sqs_receiver::client::create_sqs_client_with_credentials;
///
/// let client = create_sqs_client_with_credentials(
///     "AKIAIOSFODNN7EXAMPLE",
///     "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
///     "us-east-1"
/// );
/// ```
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
