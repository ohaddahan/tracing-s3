use crate::config::types::{
    Bucket, BufferSizeLimitKb, CronIntervalInMs, Endpoint, ObjectSizeLimitMb, Postfix, Prefix,
};
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_types::region::Region;
use dotenv::dotenv;
use std::env;

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
        buffer_size_limit_mb: BufferSizeLimitKb,
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
            buffer_size_limit_kb: buffer_size_limit_mb.inner(),
        })
    }
}
