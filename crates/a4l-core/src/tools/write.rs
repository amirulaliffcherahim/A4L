use a4l_policy::ActionRequest;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{check_policy, ToolContext, ToolError};
use crate::provider::{FunctionDefinition, ToolDefinition};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "write".to_string(),
            description: "Create or overwrite a file on disk.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to file" },
                    "content": { "type": "string", "description": "Full file content to write" }
                },
                "required": ["path", "content"]
            }),
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct WriteArgs {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct WriteResult {
    pub path: String,
    pub bytes_written: usize,
}

pub fn execute(ctx: &ToolContext, args: WriteArgs) -> Result<WriteResult, ToolError> {
    check_policy(
        ctx.policy_engine,
        &ActionRequest::FileWrite {
            path: args.path.clone(),
        },
    )?;

    let file_path = ctx.repo_root.join(&args.path);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&file_path, &args.content)?;

    Ok(WriteResult {
        path: args.path,
        bytes_written: args.content.len(),
    })
}
