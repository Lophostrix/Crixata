use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub downloading: bool,
    pub percentage: f32,
    pub step: String,
    pub error: Option<String>,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            downloading: false,
            percentage: 0.0,
            step: "Idle".into(),
            error: None,
        }
    }
}

pub struct ModelManager {
    config: AppConfig,
    client: Client,
    progress: Arc<Mutex<DownloadProgress>>,
}

impl ModelManager {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            progress: Arc::new(Mutex::new(DownloadProgress::default())),
        }
    }

    pub fn is_model_present(&self) -> bool {
        self.config.model_path().exists()
    }

    pub fn is_sidecar_present(&self) -> bool {
        self.config.resolve_llama_server_bin().exists()
    }

    pub fn get_model_path(&self) -> Option<PathBuf> {
        let path = self.config.model_path();
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn get_sidecar_path(&self) -> Option<PathBuf> {
        let path = self.config.resolve_llama_server_bin();
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn get_status(&self) -> DownloadProgress {
        self.progress.lock().unwrap().clone()
    }

    pub fn set_progress(&self, percentage: f32, step: &str) {
        let mut p = self.progress.lock().unwrap();
        p.downloading = true;
        p.percentage = percentage;
        p.step = step.to_string();
        p.error = None;
    }

    pub fn set_error(&self, err: String) {
        let mut p = self.progress.lock().unwrap();
        p.downloading = false;
        p.error = Some(err);
    }

    pub fn set_completed(&self) {
        let mut p = self.progress.lock().unwrap();
        p.downloading = false;
        p.percentage = 100.0;
        p.step = "Completed".into();
        p.error = None;
    }

    /// Downloads the GGUF model from Hugging Face with progress reporting and optional resuming.
    pub async fn download_model<F>(&self, progress_callback: F) -> Result<()>
    where
        F: Fn(f32, &str) + Send + Sync + 'static,
    {
        let url = &self.config.model_url;
        let dest = self.config.model_path();
        self.set_progress(0.0, "Downloading model");
        progress_callback(0.0, "Downloading model");

        let p_callback = Arc::new(progress_callback);
        let cb_clone = Arc::clone(&p_callback);
        let progress_state = Arc::clone(&self.progress);

        let res = download_file_with_resuming(
            &self.client,
            url,
            &dest,
            "Downloading model",
            move |pct, step| {
                if let Ok(mut st) = progress_state.lock() {
                    st.downloading = true;
                    st.percentage = pct;
                    st.step = step.to_string();
                }
                cb_clone(pct, step);
            },
        )
        .await;

        if let Err(ref e) = res {
            self.set_error(e.to_string());
        }

        res
    }

    /// Downloads the llama-server executable for the current target triple.
    pub async fn download_sidecar<F>(&self, progress_callback: F) -> Result<()>
    where
        F: Fn(f32, &str) + Send + Sync + 'static,
    {
        let target = current_target_triple();
        let url = llama_server_release_url(target);
        let dest = self.config.llama_server_bin_path();
        self.set_progress(0.0, "Downloading llama-server");
        progress_callback(0.0, "Downloading llama-server");

        let p_callback = Arc::new(progress_callback);
        let cb_clone = Arc::clone(&p_callback);
        let progress_state = Arc::clone(&self.progress);

        let res = download_file_with_resuming(
            &self.client,
            &url,
            &dest,
            "Downloading llama-server",
            move |pct, step| {
                if let Ok(mut st) = progress_state.lock() {
                    st.downloading = true;
                    st.percentage = pct;
                    st.step = step.to_string();
                }
                cb_clone(pct, step);
            },
        )
        .await;

        if res.is_ok() {
            let _ = mark_executable(&dest);
        } else if let Err(ref e) = res {
            self.set_error(e.to_string());
        }

        res
    }

    /// Downloads both the llama-server binary (if missing) and the GGUF model (if missing).
    pub async fn download_all<F>(&self, progress_callback: F) -> Result<()>
    where
        F: Fn(f32, &str) + Send + Sync + 'static,
    {
        let cb = Arc::new(progress_callback);

        if !self.is_sidecar_present() {
            let cb1 = Arc::clone(&cb);
            let res = self
                .download_sidecar(move |pct, msg| {
                    // scale sidecar download to 0% - 20%
                    let scaled = pct * 0.2;
                    cb1(scaled, msg);
                })
                .await;

            if let Err(e) = res {
                eprintln!("Warning: Failed to download llama-server from release URL: {}", e);
                // Non-fatal if sidecar binary will be placed manually or already present
            }
        }

        if !self.is_model_present() {
            let cb2 = Arc::clone(&cb);
            self.download_model(move |pct, msg| {
                // scale model download to 20% - 100%
                let scaled = 20.0 + (pct * 0.8);
                cb2(scaled, msg);
            })
            .await?;
        }

        self.set_completed();
        cb(100.0, "Completed");
        Ok(())
    }
}

pub fn current_target_triple() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "x86_64-unknown-linux-gnu"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "aarch64-unknown-linux-gnu"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "x86_64-apple-darwin"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "aarch64-apple-darwin"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "x86_64-pc-windows-msvc"
    }
    #[cfg(not(any(
        all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")),
        all(target_os = "macos", any(target_arch = "x86_64", target_arch = "aarch64")),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        "unknown"
    }
}

pub fn llama_server_asset_name(target_triple: &str) -> String {
    if target_triple.contains("windows") {
        format!("llama-server-{}.exe", target_triple)
    } else {
        format!("llama-server-{}", target_triple)
    }
}

pub fn llama_server_release_url(target_triple: &str) -> String {
    let asset_name = llama_server_asset_name(target_triple);
    format!(
        "https://github.com/ggerganov/llama.cpp/releases/latest/download/{}",
        asset_name
    )
}

#[cfg(unix)]
pub fn mark_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)
}

#[cfg(not(unix))]
pub fn mark_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

async fn download_file_with_resuming<F>(
    client: &Client,
    url: &str,
    target_path: &Path,
    step_label: &str,
    progress_fn: F,
) -> Result<()>
where
    F: Fn(f32, &str),
{
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let part_path = target_path.with_extension("download.part");

    let mut existing_bytes: u64 = 0;
    if part_path.exists() {
        if let Ok(meta) = fs::metadata(&part_path) {
            existing_bytes = meta.len();
        }
    }

    let mut req = client.get(url);
    if existing_bytes > 0 {
        req = req.header("Range", format!("bytes={}-", existing_bytes));
    }

    let mut resp = req
        .send()
        .await
        .with_context(|| format!("Failed to initiate download from {}", url))?;

    let status = resp.status();
    let (mut file, mut downloaded, total_size) = if status == reqwest::StatusCode::PARTIAL_CONTENT {
        let content_len = resp.content_length().unwrap_or(0);
        let total = existing_bytes + content_len;
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&part_path)
            .await?;
        (file, existing_bytes, total)
    } else if status.is_success() {
        let total = resp.content_length().unwrap_or(0);
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&part_path)
            .await?;
        (file, 0, total)
    } else {
        anyhow::bail!("Download failed with HTTP status: {}", status);
    };

    use tokio::io::AsyncWriteExt;

    while let Some(chunk) = resp
        .chunk()
        .await
        .with_context(|| "Error reading chunk from stream")?
    {
        file.write_all(&chunk)
            .await
            .with_context(|| "Error writing to part file")?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let pct = ((downloaded as f64 / total_size as f64) * 100.0) as f32;
            progress_fn(pct.clamp(0.0, 99.9), step_label);
        } else {
            progress_fn(50.0, step_label);
        }
    }

    file.flush().await?;
    drop(file);

    if target_path.exists() {
        let _ = fs::remove_file(target_path);
    }
    fs::rename(&part_path, target_path)
        .with_context(|| format!("Failed to rename {:?} to {:?}", part_path, target_path))?;

    Ok(())
}
