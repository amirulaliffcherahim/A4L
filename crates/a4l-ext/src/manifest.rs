use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json/toml serialize/deserialize error: {0}")]
    Format(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionEntry {
    pub name: String,
    pub source: String, // path or git url
    #[serde(default)]
    pub pinned_commit: Option<String>,
    pub content_hash: String,
    pub installed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionsManifest {
    pub version: u32,
    pub extensions: HashMap<String, ExtensionEntry>,
}

impl ExtensionsManifest {
    pub fn load_from_dir(a4l_home: &Path) -> Result<Self, ManifestError> {
        let manifest_path = a4l_home.join("extensions.json");
        if !manifest_path.exists() {
            return Ok(Self {
                version: 1,
                extensions: HashMap::new(),
            });
        }

        let content = std::fs::read_to_string(manifest_path)?;
        let manifest: ExtensionsManifest =
            serde_json::from_str(&content).map_err(|e| ManifestError::Format(e.to_string()))?;
        Ok(manifest)
    }

    pub fn save_to_dir(&self, a4l_home: &Path) -> Result<(), ManifestError> {
        std::fs::create_dir_all(a4l_home)?;
        let manifest_path = a4l_home.join("extensions.json");
        let content =
            serde_json::to_string_pretty(self).map_err(|e| ManifestError::Format(e.to_string()))?;
        std::fs::write(manifest_path, content)?;
        Ok(())
    }
}
