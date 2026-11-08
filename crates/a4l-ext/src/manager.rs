use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::manifest::{ExtensionEntry, ExtensionsManifest, ManifestError};

#[derive(Debug, Error)]
pub enum ExtensionManagerError {
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("extension already installed: {0}")]
    AlreadyExists(String),
    #[error("extension not found: {0}")]
    NotFound(String),
    #[error("build error: {0}")]
    BuildFailed(String),
}

pub struct ExtensionManager {
    a4l_home: PathBuf,
}

impl ExtensionManager {
    pub fn new(a4l_home: impl AsRef<Path>) -> Self {
        Self {
            a4l_home: a4l_home.as_ref().to_path_buf(),
        }
    }

    pub fn list(&self) -> Result<Vec<ExtensionEntry>, ExtensionManagerError> {
        let manifest = ExtensionsManifest::load_from_dir(&self.a4l_home)?;
        Ok(manifest.extensions.into_values().collect())
    }

    pub fn add(&self, name: &str, source: &str) -> Result<ExtensionEntry, ExtensionManagerError> {
        let mut manifest = ExtensionsManifest::load_from_dir(&self.a4l_home)?;
        if manifest.extensions.contains_key(name) {
            return Err(ExtensionManagerError::AlreadyExists(name.to_string()));
        }

        // Hash content or source reference
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(source.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let entry = ExtensionEntry {
            name: name.to_string(),
            source: source.to_string(),
            pinned_commit: None,
            content_hash: hash,
            installed_at: Utc::now(),
        };

        manifest.extensions.insert(name.to_string(), entry.clone());
        manifest.save_to_dir(&self.a4l_home)?;

        Ok(entry)
    }

    pub fn remove(&self, name: &str) -> Result<(), ExtensionManagerError> {
        let mut manifest = ExtensionsManifest::load_from_dir(&self.a4l_home)?;
        if manifest.extensions.remove(name).is_none() {
            return Err(ExtensionManagerError::NotFound(name.to_string()));
        }
        manifest.save_to_dir(&self.a4l_home)?;
        Ok(())
    }
}
