use a4l_policy::ActionRequest;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;

use super::{check_policy, ToolContext, ToolError};
use crate::provider::{FunctionDefinition, ToolDefinition};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "bash".to_string(),
            description: "Execute a command in the project environment and return exit code, first error, and log pointer.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "cmd": { "type": "string", "description": "Shell command to run" }
                },
                "required": ["cmd"]
            }),
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct BashArgs {
    pub cmd: String,
}

#[derive(Debug, Serialize)]
pub struct BashResult {
    pub exit_code: i32,
    pub first_error: Option<String>,
    pub stdout_preview: String,
    pub log_path: String,
}

pub fn execute(ctx: &ToolContext, args: BashArgs) -> Result<BashResult, ToolError> {
    check_policy(
        ctx.policy_engine,
        &ActionRequest::Command {
            cmd: args.cmd.clone(),
        },
    )?;

    let trimmed = args.cmd.trim();
    let history_dir = ctx.repo_root.join(".a4l").join("history");
    std::fs::create_dir_all(&history_dir)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let log_rel = format!(".a4l/history/bash_{}.log", timestamp);
    let log_path = ctx.repo_root.join(&log_rel);

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd.exe");
        c.args(["/C", trimmed]);
        c
    };

    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", trimmed]);
        c
    };

    cmd.current_dir(ctx.repo_root);

    let output = cmd.output()?;
    let exit_code = output.status.code().unwrap_or(-1);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let log_content = format!(
        "COMMAND: {}\nEXIT: {}\n\n=== STDOUT ===\n{}\n\n=== STDERR ===\n{}\n",
        trimmed, exit_code, stdout, stderr
    );
    std::fs::write(&log_path, log_content)?;

    // First error line from stderr or stdout
    let first_error = if exit_code != 0 {
        stderr
            .lines()
            .chain(stdout.lines())
            .find(|l| {
                let lower = l.to_lowercase();
                lower.contains("error") || lower.contains("fatal") || lower.contains("failed")
            })
            .map(|s| s.trim().to_string())
    } else {
        None
    };

    // Output preview (capped at 5 lines / 500 chars)
    let preview: Vec<&str> = stdout.lines().take(5).collect();
    let stdout_preview = preview.join("\n");

    Ok(BashResult {
        exit_code,
        first_error,
        stdout_preview,
        log_path: log_rel,
    })
}
