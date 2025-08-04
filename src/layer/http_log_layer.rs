use crate::config::tracing_s3_config::TracingS3Config;
use crate::s3_helpers::S3Helpers;
use chrono::Local;
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;
use uuid::Uuid;

pub struct Output {
    name: String,
    size_in_bytes: Arc<AtomicU64>,
    buffer: Arc<RwLock<Vec<String>>>,
    part: Arc<AtomicU64>,
    prefix: String,
    postfix: String,
    nonce: String,
}

impl Output {
    pub fn new(prefix: &str, postfix: &str) -> Self {
        let nonce = Uuid::new_v4().to_string();
        Self {
            name: Self::gen_name(prefix, 0, postfix, &nonce),
            nonce,
            prefix: prefix.to_string(),
            postfix: postfix.to_string(),
            buffer: Arc::new(RwLock::new(Vec::new())),
            size_in_bytes: Arc::new(AtomicU64::new(0)),
            part: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn bump_part(&mut self) {
        self.part.fetch_add(1, Ordering::Relaxed);
        let name = Self::gen_name(&self.prefix, self.part(), &self.postfix, &self.nonce);
        self.update_name(&name);
    }

    pub fn gen_name(prefix: &str, part: u64, postfix: &str, nonce: &str) -> String {
        let today = Local::now().date_naive();
        format!(
            "{}/{part}/{prefix}-{nonce}.{postfix}",
            today.format("%Y-%m-%d"),
        )
    }

    pub async fn buffer_len(&self) -> u64 {
        self.buffer.read().await.len() as u64
    }

    pub async fn flush_buffer(&self) -> String {
        let payload = self.buffer.read().await.join("\n");
        self.buffer.write().await.clear();
        self.update_size_in_bytes(0);
        payload
    }

    pub async fn append_to_buffer(&self, value: String) {
        let size_in_kb = self
            .size_in_bytes
            .fetch_add(value.len() as u64, Ordering::Relaxed);
        let mut mut_buffer = self.buffer.write().await;
        mut_buffer.push(value);
        self.update_size_in_bytes(size_in_kb);
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn size_in_bytes(&self) -> u64 {
        self.size_in_bytes.load(Ordering::Relaxed)
    }

    pub fn part(&self) -> u64 {
        self.part.load(Ordering::Relaxed)
    }

    pub fn update_size_in_bytes(&self, size_in_bytes: u64) {
        self.size_in_bytes.store(size_in_bytes, Ordering::Relaxed);
    }

    pub fn update_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

pub struct HttpLogLayer {
    pub output: Arc<RwLock<Output>>,
    pub config: Arc<TracingS3Config>,
    pub handle_tx: UnboundedSender<JoinHandle<()>>,
    pub event_tx: UnboundedSender<Value>,
}

impl HttpLogLayer {
    pub fn cron_job(config: Arc<TracingS3Config>, output: Arc<RwLock<Output>>) -> JoinHandle<()> {
        let buffer_size_limit_kb = config.buffer_size_limit_kb;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(config.cron_interval_in_ms)).await;
                let buffer_len = output.read().await.buffer_len().await;
                let size_in_bytes = output.read().await.size_in_bytes();
                if buffer_len > 0 || size_in_bytes * 1_024 >= buffer_size_limit_kb {
                    let _ = HttpLogLayer::send_logs(config.clone(), output.clone()).await;
                }
            }
        })
    }

    pub fn new(config: Arc<TracingS3Config>) -> Self {
        let output = Arc::new(RwLock::new(Output::new(&config.prefix, &config.postfix)));
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
        let output_clone = output.clone();
        tokio::spawn(async move {
            while let Some(value) = event_rx.recv().await {
                if let Ok(v) = serde_json::to_string(&value) {
                    output_clone.read().await.append_to_buffer(v).await;
                }
            }
        });

        let cron_job_handle = Self::cron_job(config.clone(), output.clone());
        let _ = handle_tx.send(cron_job_handle);
        Self {
            output,
            handle_tx,
            config,
            event_tx,
        }
    }

    pub async fn send_logs(
        config: Arc<TracingS3Config>,
        output: Arc<RwLock<Output>>,
    ) -> anyhow::Result<()> {
        let payload = output.read().await.flush_buffer().await;
        let name = output.read().await.name();
        let total_size =
            S3Helpers::append_to_file(&config.aws_client, &config.bucket, &name, &payload).await?;
        if total_size > config.buffer_size_limit_kb * 1_024 {
            output.write().await.bump_part();
        }
        Ok(())
    }
}
