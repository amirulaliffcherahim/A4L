use a4l_policy::ActionRequest;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{check_policy, ToolContext, ToolError};
use crate::provider::{FunctionDefinition, ToolDefinition};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "edit".to_string(),
            description:
                "Replace an exact unique substring within a file and return the patch diff."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to file" },
                    "old_text": { "type": "string", "description": "Exact text to replace" },
                    "new_text": { "type": "string", "description": "New replacement text" }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct EditArgs {
    pub path: String,
    pub old_text: String,
    pub new_text: String,
}

#[derive(Debug, Serialize)]
pub struct EditResult {
    pub path: String,
    pub diff: String,
    pub replacements: usize,
}

pub fn execute(ctx: &ToolContext, args: EditArgs) -> Result<EditResult, ToolError> {
    check_policy(
        ctx.policy_engine,
        &ActionRequest::FileEdit {
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

    let content = std::fs::read_to_string(&file_path)?;

    let count = content.matches(&args.old_text).count();
    if count == 0 {
        return Err(ToolError::Execution(
            "old_text not found in file".to_string(),
        ));
    }
    if count > 1 {
        return Err(ToolError::Execution(format!(
            "old_text is not unique; found {} occurrences. Provide more surrounding context.",
            count
        )));
    }

    let new_content = content.replacen(&args.old_text, &args.new_text, 1);
    std::fs::write(&file_path, new_content)?;

    let diff = format!(
        "--- {}\n+++ {}\n- {}\n+ {}",
        args.path,
        args.path,
        args.old_text.replace('\n', "\n- "),
        args.new_text.replace('\n', "\n+ ")
    );

    Ok(EditResult {
        path: args.path,
        diff,
        replacements: 1,
    })
}
