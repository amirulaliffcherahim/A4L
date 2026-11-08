use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yml::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    #[serde(default)]
    pub quirks: ProviderQuirks,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderQuirks {
    #[serde(default)]
    pub stream_usage: bool,
    #[serde(default)]
    pub use_max_completion_tokens: bool,
    #[serde(default)]
    pub estimate_tokens_if_missing: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersFile {
    pub default_provider: String,
    pub providers: HashMap<String, ProviderConfig>,
}

impl ProvidersFile {
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let file: ProvidersFile = serde_yml::from_str(&content)?;
        Ok(file)
    }
}
