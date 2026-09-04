use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server_port: u16,
    pub sidecar_port: u16,
    pub model_repo: String,
    pub model_filename: String,
    pub cdn_shard_base_url: String,
    pub data_dir: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let default_data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("./data"))
            .join("crixata");

        Self {
            server_port: 4343,
            sidecar_port: 8080,
            model_repo: "bartowski/Llama-3.2-3B-Instruct-GGUF".to_string(),
            model_filename: "Llama-3.2-3B-Instruct-Q4_K_M.gguf".to_string(),
            cdn_shard_base_url: "https://cdn.jsdelivr.net/gh/Lophostrix/crixata-cache@main/shards".to_string(),
            data_dir: default_data_dir,
        }
    }
}

impl AppConfig {
    pub fn cache_db_path(&self) -> PathBuf {
        self.data_dir.join("cache.sqlite")
    }

    pub fn models_dir(&self) -> PathBuf {
        self.data_dir.join("models")
    }

    pub fn model_file_path(&self) -> PathBuf {
        self.models_dir().join(&self.model_filename)
    }
}
