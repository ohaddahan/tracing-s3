use crate::config::Config;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinHandle;

pub struct HttpLogLayer {
    pub config: Arc<Config>,
    pub buffer: Arc<RwLock<Vec<Value>>>,
    pub handle_tx: UnboundedSender<JoinHandle<()>>,
    pub event_tx: UnboundedSender<Value>,
}

impl HttpLogLayer {
    pub fn cron_job(config: Arc<Config>, buffer: Arc<RwLock<Vec<Value>>>) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let buffer_clone = buffer.clone();
                tokio::time::sleep(Duration::from_millis(config.cron_interval_in_ms as u64)).await;
                if !buffer_clone.read().await.is_empty() {
                    // let mut buffer = buffer.read().await.clone();
                    HttpLogLayer::send_logs(buffer_clone).await;
                }
            }
        })
    }

    pub fn new(config: Arc<Config>) -> Self {
        let buffer: Arc<RwLock<Vec<Value>>> = Arc::new(RwLock::new(Vec::new()));
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
        let buffer_clone = buffer.clone();
        tokio::spawn(async move {
            while let Some(value) = event_rx.recv().await {
                println!("value = {:#?}", value);
                let mut mut_buffer = buffer_clone.write().await;
                mut_buffer.push(value);
            }
        });

        let cron_job_handle = Self::cron_job(config.clone(), buffer.clone());
        let _ = handle_tx.send(cron_job_handle);
        Self {
            handle_tx,
            config,
            buffer,
            event_tx,
        }
    }

    pub async fn send_logs(buffer: Arc<RwLock<Vec<Value>>>) {
        println!("send_logs => {:#?}", buffer.read().await);
        // let r = client.post(&*url).json(&logs).send().await;
        // match r {
        //     Ok(_) => {}
        //     Err(e) => println!("Error sending logs: {:?}", e),
        // }
    }
}
