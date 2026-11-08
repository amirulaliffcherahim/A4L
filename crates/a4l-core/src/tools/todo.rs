use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{ToolContext, ToolError};
use crate::provider::{FunctionDefinition, ToolDefinition};
use crate::state::{StateStore, TodoItem, TodoStatus};

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        r#type: "function".to_string(),
        function: FunctionDefinition {
            name: "todo".to_string(),
            description: "List, add, or update task todo status (pending or done) in pipeline or freeform session.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "add", "update", "remove"],
                        "description": "Todo action"
                    },
                    "id": { "type": "string", "description": "Todo ID for update or remove" },
                    "text": { "type": "string", "description": "Todo text description for add or update" },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "done"],
                        "description": "Status claim (the model may set pending or done only; the harness alone sets verified)"
                    }
                },
                "required": ["action"]
            }),
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct TodoArgs {
    pub action: String,
    pub id: Option<String>,
    pub text: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TodoEntryView {
    pub id: String,
    pub text: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct TodoResult {
    pub items: Vec<TodoEntryView>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct FreeformTodoList {
    items: Vec<TodoItem>,
}

pub fn execute(ctx: &ToolContext, args: TodoArgs) -> Result<TodoResult, ToolError> {
    if let Some(task_id) = &ctx.active_task_id {
        // Pipeline mode: update tasks.json
        let tasks_file = ctx.repo_root.join(".a4l").join("tasks.json");
        let store = StateStore::new(tasks_file);

        let mut current_task = store
            .get_task(task_id)
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        match args.action.as_str() {
            "list" => {
                let views = current_task
                    .todos
                    .iter()
                    .map(|t| TodoEntryView {
                        id: t.id.clone(),
                        text: t.text.clone(),
                        status: format!("{:?}", t.status).to_lowercase(),
                    })
                    .collect();
                Ok(TodoResult {
                    items: views,
                    message: format!(
                        "listed {} pipeline todos for {}",
                        current_task.todos.len(),
                        task_id
                    ),
                })
            }
            "add" => {
                let text = args
                    .text
                    .ok_or_else(|| ToolError::Execution("text required for add".to_string()))?;
                let next_idx = current_task.todos.len() + 1;
                let new_id = format!("{}.{}", task_id, next_idx);
                let item = TodoItem {
                    id: new_id,
                    text,
                    status: TodoStatus::Pending,
                    verified_at: None,
                };
                current_task.todos.push(item);
                store
                    .save(&store.load().unwrap())
                    .map_err(|e| ToolError::Execution(e.to_string()))?;

                let views = current_task
                    .todos
                    .iter()
                    .map(|t| TodoEntryView {
                        id: t.id.clone(),
                        text: t.text.clone(),
                        status: format!("{:?}", t.status).to_lowercase(),
                    })
                    .collect();
                Ok(TodoResult {
                    items: views,
                    message: "added pipeline todo".to_string(),
                })
            }
            "update" => {
                let id = args
                    .id
                    .ok_or_else(|| ToolError::Execution("id required for update".to_string()))?;
                let target_status = match args.status.as_deref() {
                    Some("done") => TodoStatus::Done,
                    Some("pending") => TodoStatus::Pending,
                    Some("verified") => {
                        return Err(ToolError::Execution(
                            "models cannot mark todo as 'verified'; only harness gate execution sets verified"
                                .to_string(),
                        ));
                    }
                    _ => TodoStatus::Pending,
                };

                let updated_task = store
                    .update_task(task_id, |t| {
                        if let Some(todo) = t.todos.iter_mut().find(|it| it.id == id) {
                            // If it's already verified by harness, model cannot downgrade it to pending or done
                            if todo.status != TodoStatus::Verified {
                                todo.status = target_status;
                            }
                            if let Some(new_text) = args.text {
                                todo.text = new_text;
                            }
                        }
                        Ok(())
                    })
                    .map_err(|e| ToolError::Execution(e.to_string()))?;

                let views = updated_task
                    .todos
                    .iter()
                    .map(|t| TodoEntryView {
                        id: t.id.clone(),
                        text: t.text.clone(),
                        status: format!("{:?}", t.status).to_lowercase(),
                    })
                    .collect();

                Ok(TodoResult {
                    items: views,
                    message: format!("updated todo {}", id),
                })
            }
            "remove" => {
                let id = args
                    .id
                    .ok_or_else(|| ToolError::Execution("id required for remove".to_string()))?;
                let updated_task = store
                    .update_task(task_id, |t| {
                        t.todos.retain(|it| it.id != id);
                        Ok(())
                    })
                    .map_err(|e| ToolError::Execution(e.to_string()))?;

                let views = updated_task
                    .todos
                    .iter()
                    .map(|t| TodoEntryView {
                        id: t.id.clone(),
                        text: t.text.clone(),
                        status: format!("{:?}", t.status).to_lowercase(),
                    })
                    .collect();

                Ok(TodoResult {
                    items: views,
                    message: format!("removed todo {}", id),
                })
            }
            other => Err(ToolError::Execution(format!("unknown action: {}", other))),
        }
    } else {
        // Freeform session mode: .a4l/session/todos.json
        let session_dir = ctx.repo_root.join(".a4l").join("session");
        std::fs::create_dir_all(&session_dir)?;
        let session_file = session_dir.join("todos.json");

        let mut list: FreeformTodoList = if session_file.exists() {
            let data = std::fs::read_to_string(&session_file)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            FreeformTodoList::default()
        };

        match args.action.as_str() {
            "list" => {}
            "add" => {
                let text = args
                    .text
                    .ok_or_else(|| ToolError::Execution("text required for add".to_string()))?;
                let next_id = (list.items.len() + 1).to_string();
                list.items.push(TodoItem {
                    id: next_id,
                    text,
                    status: TodoStatus::Pending,
                    verified_at: None,
                });
                let json_data = serde_json::to_string_pretty(&list)?;
                std::fs::write(&session_file, json_data)?;
            }
            "update" => {
                let id = args
                    .id
                    .ok_or_else(|| ToolError::Execution("id required for update".to_string()))?;
                if let Some(item) = list.items.iter_mut().find(|it| it.id == id) {
                    if let Some("done") = args.status.as_deref() {
                        item.status = TodoStatus::Done;
                    } else if let Some("pending") = args.status.as_deref() {
                        item.status = TodoStatus::Pending;
                    }
                    if let Some(txt) = args.text {
                        item.text = txt;
                    }
                }
                let json_data = serde_json::to_string_pretty(&list)?;
                std::fs::write(&session_file, json_data)?;
            }
            "remove" => {
                let id = args
                    .id
                    .ok_or_else(|| ToolError::Execution("id required for remove".to_string()))?;
                list.items.retain(|it| it.id != id);
                let json_data = serde_json::to_string_pretty(&list)?;
                std::fs::write(&session_file, json_data)?;
            }
            other => return Err(ToolError::Execution(format!("unknown action: {}", other))),
        }

        let views = list
            .items
            .iter()
            .map(|t| TodoEntryView {
                id: t.id.clone(),
                text: t.text.clone(),
                status: format!("{:?}", t.status).to_lowercase(),
            })
            .collect();

        Ok(TodoResult {
            items: views,
            message: "freeform todos updated".to_string(),
        })
    }
}
