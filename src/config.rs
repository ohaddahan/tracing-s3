use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_types::region::Region;
use dotenv::dotenv;
use std::env;

#[derive(Debug)]
pub struct CurrentLogObject {
    pub path: String,
    pub size: usize,
}

#[derive(Debug)]
pub struct Config {
    pub aws_client: Client,
    pub bucket: String,
    pub prefix: String,
    pub object_size_limit_mb: usize,
    pub cron_interval_in_ms: usize,
}

pub struct ObjectSizeLimitMb(pub usize);
pub struct CronIntervalInMs(pub usize);
pub struct Bucket<'a>(pub Option<&'a str>);
pub struct Prefix<'a>(pub &'a str);
pub struct Endpoint<'a>(pub Option<&'a str>);

impl Config {
    #[allow(clippy::too_many_arguments)]
    pub async fn new<'a>(
        aws_region: Option<&str>,
        aws_access_key: Option<&str>,
        aws_secret_access_key: Option<&str>,
        bucket: Bucket<'a>,
        prefix: Prefix<'a>,
        endpoint: Endpoint<'a>,
        object_size_limit_mb: ObjectSizeLimitMb,
        cron_interval_in_ms: CronIntervalInMs,
    ) -> anyhow::Result<Self> {
        dotenv().ok();
        let region = Region::new(
            aws_region
                .unwrap_or(&env::var("S3_TRACING_AWS_REGION").unwrap_or("us-east-1".to_string()))
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
        // let cred_provider = SharedCredentialsProvider::new(credentials);
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
            object_size_limit_mb: object_size_limit_mb.0,
            cron_interval_in_ms: cron_interval_in_ms.0,
        })
    }
}
