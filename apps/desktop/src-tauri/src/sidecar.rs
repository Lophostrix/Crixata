use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::{Context, Result};
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
    server_bin: PathBuf,
    model_path: PathBuf,
    port: u16,
    status: Arc<Mutex<SidecarStatus>>,
    child: Option<Child>,
    client: Client,
    max_retries: u32,
}

impl SidecarManager {
    pub fn new(server_bin: PathBuf, model_path: PathBuf, port: u16) -> Self {
        Self {
            server_bin,
            model_path,
            port,
            status: Arc::new(Mutex::new(SidecarStatus::Stopped)),
            child: None,
            client: Client::builder()
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap_or_default(),
            max_retries: 3,
        }
    }

    pub fn update_paths(&mut self, server_bin: PathBuf, model_path: PathBuf) {
        self.server_bin = server_bin;
        self.model_path = model_path;
    }

    pub fn status(&self) -> SidecarStatus {
        *self.status.lock().unwrap()
    }

    pub fn is_ready(&self) -> bool {
        self.status() == SidecarStatus::Ready
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn check_health(&self) -> bool {
        let url = format!("http://127.0.0.1:{}/health", self.port);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Spawns `llama-server` and waits for `/health` endpoint to respond before marking Ready.
    /// Retries up to `max_retries` on failure.
    pub async fn start(&mut self) -> Result<()> {
        if self.is_ready() && self.check_health().await {
            return Ok(());
        }

        if !self.server_bin.exists() {
            *self.status.lock().unwrap() = SidecarStatus::Error;
            anyhow::bail!(
                "llama-server executable not found at {:?}. Please download it first.",
                self.server_bin
            );
        }

        if !self.model_path.exists() {
            *self.status.lock().unwrap() = SidecarStatus::Error;
            anyhow::bail!(
                "GGUF model not found at {:?}. Please download it first.",
                self.model_path
            );
        }

        *self.status.lock().unwrap() = SidecarStatus::Starting;

        for attempt in 1..=self.max_retries {
            // Clean up any stale process
            self.stop().ok();
            *self.status.lock().unwrap() = SidecarStatus::Starting;

            let mut cmd = Command::new(&self.server_bin);
            cmd.arg("-m")
                .arg(&self.model_path)
                .arg("--port")
                .arg(self.port.to_string())
                .arg("-c")
                .arg("4096")
                .arg("--host")
                .arg("127.0.0.1");

            let child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "Attempt {}/{}: Failed to spawn llama-server: {}",
                        attempt, self.max_retries, e
                    );
                    if attempt == self.max_retries {
                        *self.status.lock().unwrap() = SidecarStatus::Error;
                        return Err(e).context("Failed to spawn llama-server process");
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };

            self.child = Some(child);

            // Wait for /health endpoint to become responsive
            let mut ready = false;
            for _ in 0..30 {
                tokio::time::sleep(Duration::from_millis(300)).await;

                // Check if process crashed early
                if let Some(ref mut c) = self.child {
                    if let Ok(Some(exit_status)) = c.try_wait() {
                        eprintln!(
                            "llama-server exited prematurely with status: {:?}",
                            exit_status
                        );
                        break;
                    }
                }

                if self.check_health().await {
                    ready = true;
                    break;
                }
            }

            if ready {
                *self.status.lock().unwrap() = SidecarStatus::Ready;
                return Ok(());
            }

            eprintln!(
                "Attempt {}/{}: llama-server failed to respond to /health.",
                attempt, self.max_retries
            );
        }

        self.stop().ok();
        *self.status.lock().unwrap() = SidecarStatus::Error;
        anyhow::bail!(
            "llama-server failed to become ready after {} attempts.",
            self.max_retries
        )
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        *self.status.lock().unwrap() = SidecarStatus::Stopped;
        Ok(())
    }
}

impl Drop for SidecarManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
