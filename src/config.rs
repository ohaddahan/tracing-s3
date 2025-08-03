use aws_config::BehaviorVersion;
use aws_config::meta::region::RegionProviderChain;
use aws_credential_types::Credentials;
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_sdk_s3::Client;
use aws_types::region::Region;
use std::env;

pub struct Config {
    pub aws_client: Client,
    pub bucket: String,
    pub prefix: String,
    pub object_size_limit_mb: usize,
    pub cron_interval_in_ms: usize,
}

impl Config {
    pub async fn new(
        aws_region: Option<&str>,
        aws_access_key: Option<&str>,
        aws_secret_access_key: Option<&str>,
        bucket: &str,
        prefix: &str,
        object_size_limit_mb: usize,
        cron_interval_in_ms: usize,
    ) -> anyhow::Result<Self> {
        let region = Region::new(
            aws_region
                .unwrap_or(&env::var("S3_TRACING_AWS_REGION").unwrap_or("us-east-1".to_string()))
                .to_string(),
        );
        let region_provider = RegionProviderChain::first_try(region);
        let shared_creds = SharedCredentialsProvider::new(Credentials::from_keys(
            match aws_access_key {
                Some(access_key) => access_key.to_string(),
                None => env::var("S3_TRACING_AWS_ACCESS_KEY_ID")?,
            },
            match aws_secret_access_key {
                Some(secret_access_key) => secret_access_key.to_string(),
                None => env::var("S3_TRACING_AWS_SECRET_ACCESS_KEY")?,
            },
            None,
        ));
        let shared_config = aws_config::from_env()
            .behavior_version(BehaviorVersion::latest())
            .credentials_provider(shared_creds)
            .region(region_provider)
            .load()
            .await;
        let aws_client = Client::new(&shared_config);
        Ok(Self {
            aws_client,
            bucket: bucket.to_string(),
            prefix: prefix.to_string(),
            object_size_limit_mb,
            cron_interval_in_ms,
        })
    }
}
