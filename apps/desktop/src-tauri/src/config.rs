use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server_port: u16,
    pub llama_server_port: u16,
    pub model_repo: String,
    pub model_filename: String,
    pub model_url: String,
    pub cdn_shard_base_url: String,
    pub data_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let base_data_dir = dirs::data_local_dir()
            .or_else(dirs::data_dir)
            .unwrap_or_else(|| PathBuf::from("./data"));
        let data_dir = base_data_dir.join("crixata");

        Self {
            server_port: 4343,
            llama_server_port: 4344,
            model_repo: "bartowski/Llama-3.2-3B-Instruct-GGUF".to_string(),
            model_filename: "Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            model_url: "https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            cdn_shard_base_url: "https://cdn.jsdelivr.net/gh/Lophostrix/crixata-cache@main/shards".to_string(),
            data_dir,
        }
    }
}

impl AppConfig {
    /// Ensures that all required application directories exist on the filesystem.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(self.data_dir.join("models"))?;
        fs::create_dir_all(self.data_dir.join("bin"))?;
        Ok(())
    }

    /// Path to the SQLite cache database.
    pub fn cache_db_path(&self) -> PathBuf {
        let _ = self.ensure_dirs();
        self.data_dir.join("cache.sqlite")
    }

    /// Directory for downloaded GGUF models.
    pub fn model_dir(&self) -> PathBuf {
        let dir = self.data_dir.join("models");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Alias for model_dir for backwards compatibility.
    pub fn models_dir(&self) -> PathBuf {
        self.model_dir()
    }

    /// Absolute path to the model file.
    pub fn model_path(&self) -> PathBuf {
        let primary = self.model_dir().join(&self.model_filename);
        if primary.exists() {
            return primary;
        }

        // Support alternative dot notation
        let alt = self.model_dir().join("Llama-3.2-3B-Instruct.Q4_K_M.gguf");
        if alt.exists() {
            return alt;
        }

        primary
    }

    /// Path to downloaded llama-server executable in the app data directory.
    pub fn llama_server_bin_path(&self) -> PathBuf {
        let bin_dir = self.data_dir.join("bin");
        let _ = fs::create_dir_all(&bin_dir);

        #[cfg(target_os = "windows")]
        let bin_name = "llama-server.exe";
        #[cfg(not(target_os = "windows"))]
        let bin_name = "llama-server";

        bin_dir.join(bin_name)
    }

    /// Resolves the llama-server executable, checking app data dir first,
    /// then checking the local sidecars/ directory if present.
    pub fn resolve_llama_server_bin(&self) -> PathBuf {
        let standard_bin = self.llama_server_bin_path();
        if standard_bin.exists() {
            return standard_bin;
        }

        // Also check repo sidecars directory if running from source
        let sidecars_dir = Path::new("sidecars");
        if sidecars_dir.exists() {
            #[cfg(target_os = "windows")]
            let candidate = sidecars_dir.join("llama-server.exe");
            #[cfg(not(target_os = "windows"))]
            let candidate = sidecars_dir.join("llama-server");

            if candidate.exists() {
                return candidate;
            }

            let triple = crate::model::current_target_triple();
            #[cfg(target_os = "windows")]
            let triple_candidate = sidecars_dir.join(format!("llama-server-{}.exe", triple));
            #[cfg(not(target_os = "windows"))]
            let triple_candidate = sidecars_dir.join(format!("llama-server-{}", triple));

            if triple_candidate.exists() {
                return triple_candidate;
            }
        }

        standard_bin
    }
}
