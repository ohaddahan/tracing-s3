use crate::config::tracing_s3_config::TracingS3Config;
use crate::s3_helpers::S3Helpers;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;

pub struct HttpLogLayer {
    pub config: Arc<TracingS3Config>,
    pub buffer: Arc<RwLock<Vec<String>>>,
    pub buffer_size_bytes: Arc<AtomicU64>,
    pub handle_tx: UnboundedSender<JoinHandle<()>>,
    pub event_tx: UnboundedSender<Value>,
}

impl HttpLogLayer {
    pub fn cron_job(
        config: Arc<TracingS3Config>,
        buffer: Arc<RwLock<Vec<String>>>,
        buffer_size_bytes: Arc<AtomicU64>,
        buffer_size_limit_mb: u64,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(config.cron_interval_in_ms)).await;
                let buffer_len = buffer.read().await.len();
                if buffer_len > 0
                    || buffer_size_bytes.load(Ordering::Relaxed) * 1_000 > buffer_size_limit_mb
                {
                    HttpLogLayer::send_logs(
                        config.clone(),
                        buffer.clone(),
                        buffer_size_bytes.clone(),
                    )
                    .await;
                }
            }
        })
    }

    pub fn new(config: Arc<TracingS3Config>) -> Self {
        let buffer: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
        let (handle_tx, mut handle_rx): (
            UnboundedSender<JoinHandle<()>>,
            UnboundedReceiver<JoinHandle<()>>,
        ) = mpsc::unbounded_channel();

        let (event_tx, mut event_rx): (UnboundedSender<Value>, UnboundedReceiver<Value>) =
            mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(handle) = handle_rx.recv().await {
                let _ = handle.await;
            }
        });
        let buffer_size_bytes = Arc::new(AtomicU64::new(0));
        let buffer_size_bytes_clone = buffer_size_bytes.clone();
        let buffer_clone = buffer.clone();
        tokio::spawn(async move {
            while let Some(value) = event_rx.recv().await {
                if let Ok(v) = serde_json::to_string(&value) {
                    let mut mut_buffer = buffer_clone.write().await;
                    buffer_size_bytes_clone.fetch_add(v.len() as u64, Ordering::Relaxed);
                    mut_buffer.push(v);
                }
            }
        });

        let cron_job_handle = Self::cron_job(
            config.clone(),
            buffer.clone(),
            buffer_size_bytes.clone(),
            config.buffer_size_limit_mb,
        );
        let _ = handle_tx.send(cron_job_handle);
        Self {
            handle_tx,
            config,
            buffer,
            event_tx,
            buffer_size_bytes,
        }
    }

    pub async fn send_logs(
        config: Arc<TracingS3Config>,
        buffer: Arc<RwLock<Vec<String>>>,
        buffer_size_bytes: Arc<AtomicU64>,
    ) {
        let payload = buffer.read().await.join("\n");
        buffer.write().await.clear();
        let _ = S3Helpers::append_to_file_multipart(
            &config.aws_client,
            &config.bucket,
            &config.prefix,
            &payload,
        )
        .await;
        buffer_size_bytes.store(0, Ordering::Relaxed);
    }
}
