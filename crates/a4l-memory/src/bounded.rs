use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct BoundedMemoryManager {
    file_path: PathBuf,
    max_lines: usize,
    max_bytes: usize,
}

impl BoundedMemoryManager {
    pub fn new(file_path: impl AsRef<Path>, max_lines: usize) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            max_lines,
            max_bytes: 32_768, // 32 KB hard cap
        }
    }

    pub fn with_byte_limit(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    pub fn append_entry(&self, entry: &str) -> Result<(), MemoryError> {
        let existing = if self.file_path.exists() {
            std::fs::read_to_string(&self.file_path)?
        } else {
            String::new()
        };

        let mut lines: Vec<String> = existing.lines().map(|s| s.to_string()).collect();

        // Add new entry lines
        for line in entry.lines() {
            lines.push(line.to_string());
        }

        // Enforce hard line limit: retain header and newest lines
        if lines.len() > self.max_lines {
            let keep_from = lines.len().saturating_sub(self.max_lines);
            let has_header = lines.first().map(|s| s.starts_with("# ")).unwrap_or(false);
            if has_header {
                let header = lines[0].clone();
                lines = lines[keep_from..].to_vec();
                lines.insert(0, header);
            } else {
                lines = lines[keep_from..].to_vec();
            }
        }

        // Enforce hard byte limit: trim oldest non-header lines until under max_bytes
        let has_header = lines.first().map(|s| s.starts_with("# ")).unwrap_or(false);
        while lines.len() > 1 && lines.iter().map(|l| l.len() + 1).sum::<usize>() > self.max_bytes {
            if has_header && lines.len() > 2 {
                lines.remove(1); // remove oldest line after header
            } else {
                lines.remove(0);
            }
        }

        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)?;

        for line in lines {
            writeln!(file, "{}", line)?;
        }

        Ok(())
    }

    pub fn read_memory(&self) -> Result<String, MemoryError> {
        if !self.file_path.exists() {
            return Ok(String::new());
        }
        Ok(std::fs::read_to_string(&self.file_path)?)
    }
}
