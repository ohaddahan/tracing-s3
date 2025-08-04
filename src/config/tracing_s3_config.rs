use crate::config::types::{
    Bucket, BufferSizeLimitKb, CronIntervalInMs, Endpoint, ObjectSizeLimitMb, Postfix, Prefix,
};
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_types::region::Region;
use dotenv::dotenv;
use std::env;

/// Configuration for the S3 tracing layer.
/// Contains all necessary information to connect to AWS S3 and configure logging behavior.
#[derive(Debug)]
pub struct TracingS3Config {
    pub aws_client: Client,
    pub bucket: String,
    pub prefix: String,
    pub postfix: String,
    pub object_size_limit_mb: u64,
    pub cron_interval_in_ms: u64,
    pub buffer_size_limit_kb: u64,
}

impl TracingS3Config {
    /// Creates a new TracingS3Config instance with the provided parameters.
    ///
    /// This method will attempt to load missing configuration from environment variables:
    /// - `S3_TRACING_AWS_REGION` for the AWS region (defaults to "us-west-2")
    /// - `S3_TRACING_BUCKET` for the S3 bucket name
    /// - `S3_TRACING_AWS_ACCESS_KEY_ID` for the AWS access key
    /// - `S3_TRACING_AWS_SECRET_ACCESS_KEY` for the AWS secret key
    ///
    /// # Arguments
    /// * `aws_region` - Optional AWS region override
    /// * `aws_access_key` - Optional AWS access key override
    /// * `aws_secret_access_key` - Optional AWS secret access key override
    /// * `bucket` - S3 bucket configuration
    /// * `prefix` - Log file prefix
    /// * `postfix` - Log file postfix/extension
    /// * `endpoint` - Optional custom S3 endpoint
    /// * `object_size_limit_mb` - Maximum size for log files in MB
    /// * `cron_interval_in_ms` - Interval for flushing logs in milliseconds
    /// * `buffer_size_limit_kb` - Buffer size limit in KB
    ///
    /// # Returns
    /// * `Ok(TracingS3Config)` - If configuration is valid and AWS client can be created
    /// * `Err(anyhow::Error)` - If configuration is invalid or AWS client creation fails
    #[allow(clippy::too_many_arguments)]
    pub async fn new<'a>(
        aws_region: Option<&str>,
        aws_access_key: Option<&str>,
        aws_secret_access_key: Option<&str>,
        bucket: Bucket<'a>,
        prefix: Prefix<'a>,
        postfix: Postfix<'a>,
        endpoint: Endpoint<'a>,
        object_size_limit_mb: ObjectSizeLimitMb,
        cron_interval_in_ms: CronIntervalInMs,
        buffer_size_limit_kb: BufferSizeLimitKb,
    ) -> anyhow::Result<Self> {
        dotenv().ok();
        let region = Region::new(
            aws_region
                .unwrap_or(&env::var("S3_TRACING_AWS_REGION").unwrap_or("us-west-2".to_string()))
                .to_string(),
        );
        let bucket = match bucket.0 {
            Some(bucket) => bucket.to_string(),
            None => env::var("S3_TRACING_BUCKET")?,
        };
        let aws_access_key = match aws_access_key {
            Some(access_key) => access_key.to_string(),
            None => env::var("S3_TRACING_AWS_ACCESS_KEY_ID")?,
        };
        let aws_secret_access_key = match aws_secret_access_key {
            Some(secret_access_key) => secret_access_key.to_string(),
            None => env::var("S3_TRACING_AWS_SECRET_ACCESS_KEY")?,
        };
        let credentials =
            Credentials::new(aws_access_key, aws_secret_access_key, None, None, "AWS");
        let mut config_builder = aws_sdk_s3::Config::builder()
            .behavior_version_latest()
            .credentials_provider(credentials.clone())
            .express_credentials_provider(credentials)
            .region(region);
        config_builder = match endpoint.0 {
            Some(endpoint) => config_builder.endpoint_url(endpoint),
            None => config_builder,
        };
        let config = config_builder.build();
        let aws_client = Client::from_conf(config);
        Ok(Self {
            aws_client,
            bucket,
            prefix: prefix.0.to_string(),
            postfix: postfix.0.to_string(),
            object_size_limit_mb: object_size_limit_mb.inner(),
            cron_interval_in_ms: cron_interval_in_ms.inner(),
            buffer_size_limit_kb: buffer_size_limit_kb.inner(),
        })
    }
}
