use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FenceError {
    #[error("command execution error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git error: {0}")]
    Git(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum FenceStatus {
    Clean,
    Tampered { changed_files: Vec<String> },
}

pub fn check_git_test_fence(
    repo_path: &Path,
    locked_commit: Option<&str>,
    test_path: &str,
) -> Result<FenceStatus, FenceError> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);

    if let Some(commit) = locked_commit {
        cmd.args(["diff", "--name-only", commit, "--", test_path]);
    } else {
        cmd.args(["diff", "--name-only", "HEAD", "--", test_path]);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FenceError::Git(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let changed_files: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if changed_files.is_empty() {
        Ok(FenceStatus::Clean)
    } else {
        Ok(FenceStatus::Tampered { changed_files })
    }
}

pub fn revert_test_fence(repo_path: &Path, test_path: &str) -> Result<(), FenceError> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["checkout", "HEAD", "--", test_path])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(FenceError::Git(stderr.to_string()));
    }
    Ok(())
}
