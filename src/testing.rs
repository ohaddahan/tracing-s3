#[cfg(test)]
mod tests {
    use crate::config::tracing_s3_config::TracingS3Config;
    use crate::config::types::{
        Bucket, BufferSizeLimitKb, CronIntervalInMs, Endpoint, ObjectSizeLimitMb, Postfix, Prefix,
    };
    use crate::layer::http_log_layer::HttpLogLayer;
    use std::sync::Arc;
    use std::time::Duration;
    use tracing::Dispatch;
    use tracing::dispatcher::with_default;
    use tracing_subscriber::layer::SubscriberExt;

    #[tracing::instrument(name = "add", skip_all)]
    pub fn add(left: u64, right: u64) -> u64 {
        left + right
    }

    #[tokio::test]
    async fn json_tracing() {
        use tracing_subscriber::fmt::format::FmtSpan;
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_span_events(FmtSpan::CLOSE)
            .with_max_level(tracing::Level::INFO) // Set desired log level
            .finish();
        with_default(&Dispatch::new(subscriber), || {
            let result = add(2, 2);
            assert_eq!(result, 4);
        });
        tokio::time::sleep(Duration::from_millis(1_000)).await;
    }

    #[tokio::test]
    async fn http_tracing() {
        let config = TracingS3Config::new(
            None,
            None,
            None,
            Bucket(None),
            Prefix("prefix"),
            Postfix("log"),
            Endpoint(None),
            ObjectSizeLimitMb::new(1).unwrap(),
            CronIntervalInMs::new(1_000).unwrap(),
            BufferSizeLimitKb::new(1).unwrap(),
        )
        .await
        .unwrap();
        let http_log_layer = HttpLogLayer::new(Arc::new(config));
        let subscriber = tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with(tracing_subscriber::fmt::layer().with_ansi(false))
            .with(http_log_layer);
        with_default(&Dispatch::new(subscriber), || {
            tracing::info!("Info test");
            tracing::warn!("Warn test");
            tracing::debug!("Debug test");
            tracing::error!("Error test");
            for i in 0..500 {
                let result = add(2, i);
                assert_eq!(result, 2 + i);
            }
        });
        tokio::time::sleep(Duration::from_millis(10_000)).await;
    }
}
