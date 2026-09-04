use std::path::{Path, PathBuf};

pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    pub fn new(models_dir: &Path) -> Self {
        Self {
            models_dir: models_dir.to_path_buf(),
        }
    }

    pub fn is_model_present(&self, filename: &str) -> bool {
        self.models_dir.join(filename).exists()
    }

    pub fn get_model_path(&self, filename: &str) -> Option<PathBuf> {
        let path = self.models_dir.join(filename);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    pub fn huggingface_url(repo: &str, filename: &str) -> String {
        format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo, filename
        )
    }
}
