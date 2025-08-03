#[cfg(test)]
mod tests {
    use crate::config::{Bucket, Config, CronIntervalInMs, Endpoint, ObjectSizeLimitMb, Prefix};
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
        let config = Config::new(
            None,
            None,
            None,
            Bucket(None),
            Prefix("prefix"),
            Endpoint(None),
            ObjectSizeLimitMb(1),
            CronIntervalInMs(1_000),
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
            let result = add(2, 2);
            assert_eq!(result, 4);
        });
        tokio::time::sleep(Duration::from_millis(1_000)).await;
    }
}
