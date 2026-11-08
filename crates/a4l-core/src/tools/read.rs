use a4l_policy::ActionRequest;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{check_policy, ToolContext, ToolError};
use crate::provider::{FunctionDefinition, ToolDefinition};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "read".to_string(),
            description: "Read a file from disk with optional line offset and limit.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to file" },
                    "offset": { "type": "integer", "description": "1-based starting line number" },
                    "limit": { "type": "integer", "description": "Maximum number of lines to return" }
                },
                "required": ["path"]
            }),
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadArgs {
    pub path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ReadResult {
    pub content: String,
    pub lines_read: usize,
    pub total_lines: usize,
    pub continuation_hint: Option<String>,
}

pub fn execute(ctx: &ToolContext, args: ReadArgs) -> Result<ReadResult, ToolError> {
    check_policy(
        ctx.policy_engine,
        &ActionRequest::FileRead {
            path: args.path.clone(),
        },
    )?;

    let file_path = ctx.repo_root.join(&args.path);
    if !file_path.exists() {
        return Err(ToolError::Execution(format!(
            "file not found: {}",
            args.path
        )));
    }

    let full_content = std::fs::read_to_string(file_path)?;
    let all_lines: Vec<&str> = full_content.lines().collect();
    let total_lines = all_lines.len();

    let start = args.offset.unwrap_or(1).saturating_sub(1);
    let max_lines = args.limit.unwrap_or(200);

    let end = (start + max_lines).min(total_lines);
    let selected = if start < total_lines {
        all_lines[start..end].join("\n")
    } else {
        String::new()
    };

    let continuation_hint = if end < total_lines {
        Some(format!(
            "File has {} more lines; call read with offset={}",
            total_lines - end,
            end + 1
        ))
    } else {
        None
    };

    Ok(ReadResult {
        content: selected,
        lines_read: end.saturating_sub(start),
        total_lines,
        continuation_hint,
    })
}
