use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{ToolContext, ToolError};
use crate::provider::{FunctionDefinition, ToolDefinition};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "ask_user".to_string(),
            description: "Ask the user 1-4 structured questions with options when a decision or clarification is needed.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": { "type": "string" },
                                "header": { "type": "string" },
                                "multi_select": { "type": "boolean" },
                                "options": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "label": { "type": "string" },
                                            "description": { "type": "string" }
                                        },
                                        "required": ["label", "description"]
                                    }
                                }
                            },
                            "required": ["question", "header", "options"]
                        }
                    }
                },
                "required": ["questions"]
            }),
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct AskUserQuestion {
    pub question: String,
    pub header: String,
    #[serde(default)]
    pub multi_select: bool,
    pub options: Vec<AskUserOption>,
}

#[derive(Debug, Deserialize)]
pub struct AskUserOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct AskUserArgs {
    pub questions: Vec<AskUserQuestion>,
}

#[derive(Debug, Serialize)]
pub struct AskUserResult {
    pub answers: Vec<String>,
    pub status: String, // "answered" | "no_channel"
}

pub fn execute(ctx: &ToolContext, args: AskUserArgs) -> Result<AskUserResult, ToolError> {
    if ctx.is_headless {
        return Ok(AskUserResult {
            answers: Vec::new(),
            status: "no interactive channel available (headless mode)".to_string(),
        });
    }

    // In CLI terminal interactive mode, print questions and read line
    let mut answers = Vec::new();
    for q in &args.questions {
        println!("\n[QUESTION] {}: {}", q.header, q.question);
        for (i, opt) in q.options.iter().enumerate() {
            println!("  {}) {} — {}", i + 1, opt.label, opt.description);
        }
        print!("> ");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        answers.push(input.trim().to_string());
    }

    Ok(AskUserResult {
        answers,
        status: "answered".to_string(),
    })
}
