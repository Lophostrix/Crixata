use std::path::Path;
use std::process::Child;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidecarStatus {
    Stopped,
    Starting,
    Ready,
    Error,
}

pub struct SidecarManager {
    port: u16,
    status: Arc<Mutex<SidecarStatus>>,
    child: Option<Child>,
    client: Client,
}

impl SidecarManager {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            status: Arc::new(Mutex::new(SidecarStatus::Stopped)),
            child: None,
            client: Client::new(),
        }
    }

    pub fn status(&self) -> SidecarStatus {
        *self.status.lock().unwrap()
    }

    pub fn is_ready(&self) -> bool {
        self.status() == SidecarStatus::Ready
    }

    pub fn start(&mut self, _model_path: &Path) -> Result<()> {
        // In full implementation, uses Tauri sidecar command or std::process::Command
        // to spawn `llama-server -m <model_path> --port <port> -c 4096`
        *self.status.lock().unwrap() = SidecarStatus::Ready;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
        *self.status.lock().unwrap() = SidecarStatus::Stopped;
        Ok(())
    }

    pub async fn check_health(&self) -> bool {
        let url = format!("http://127.0.0.1:{}/health", self.port);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}
