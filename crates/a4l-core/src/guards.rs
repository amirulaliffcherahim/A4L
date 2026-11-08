use thiserror::Error;

use crate::state::{CircuitBreakerStatus, Task, TaskRole, TaskStatus, TodoStatus};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GuardFailure {
    #[error("invalid transition from role {from_role} to {to_role}")]
    InvalidRoleTransition {
        from_role: TaskRole,
        to_role: TaskRole,
    },
    #[error("invalid status transition from {from_status:?} to {to_status:?}")]
    InvalidStatusTransition {
        from_status: TaskStatus,
        to_status: TaskStatus,
    },
    #[error("context bundle missing for task {task_id}")]
    MissingContextBundle { task_id: String },
    #[error("todos list is empty for task {task_id}")]
    EmptyTodos { task_id: String },
    #[error("acceptance criteria missing test commands (L2 or L4)")]
    MissingGateCommands,
    #[error("todo '{todo_id}' is not verified (status: {status:?})")]
    TodoNotVerified { todo_id: String, status: TodoStatus },
    #[error("gate '{gate}' did not pass with exit 0 (exit: {exit:?})")]
    GateFailed { gate: String, exit: Option<i32> },
    #[error("circuit breaker is tripped: {reason:?}")]
    CircuitBreakerTripped { reason: String },
    #[error("error hash '{hash}' repeated {count} times (max allowed 1 before breaker)")]
    DuplicateErrorThreshold { hash: String, count: u32 },
    #[error("only human executive can mark task completed or send back")]
    UnauthorizedCompletion,
}

pub fn can_advance(
    task: &Task,
    target_role: TaskRole,
    target_status: TaskStatus,
) -> Result<(), GuardFailure> {
    let current_role = task.state.role;
    let current_status = task.state.status;

    // Check circuit breaker first
    if task.circuit_breaker.status == CircuitBreakerStatus::Tripped {
        return Err(GuardFailure::CircuitBreakerTripped {
            reason: format!("{:?}", task.circuit_breaker.reason),
        });
    }

    match (current_role, target_role) {
        // Planner -> Maker
        (TaskRole::Planner, TaskRole::Maker) => {
            if target_status != TaskStatus::InProgress {
                return Err(GuardFailure::InvalidStatusTransition {
                    from_status: current_status,
                    to_status: target_status,
                });
            }

            // Context bundle must exist
            match &task.state.context_bundle {
                Some(bundle) if !bundle.trim().is_empty() => {}
                _ => {
                    return Err(GuardFailure::MissingContextBundle {
                        task_id: task.id.clone(),
                    });
                }
            }

            // Todos non-empty
            if task.todos.is_empty() {
                return Err(GuardFailure::EmptyTodos {
                    task_id: task.id.clone(),
                });
            }

            // Gates must have L2 or L4 commands configured if ACs exist
            if !task.acceptance_criteria.is_empty()
                && task.gates.l2.cmd.trim().is_empty()
                && task.gates.l4.cmd.trim().is_empty()
            {
                return Err(GuardFailure::MissingGateCommands);
            }

            Ok(())
        }

        // Maker -> Checker
        (TaskRole::Maker, TaskRole::Checker) => {
            if target_status != TaskStatus::Review {
                return Err(GuardFailure::InvalidStatusTransition {
                    from_status: current_status,
                    to_status: target_status,
                });
            }

            // Every todo must be Verified (harness exit 0 proof, not model claim)
            for todo in &task.todos {
                if todo.status != TodoStatus::Verified {
                    return Err(GuardFailure::TodoNotVerified {
                        todo_id: todo.id.clone(),
                        status: todo.status,
                    });
                }
            }

            // All 4 gates must have exit == Some(0)
            let gates = [
                ("L1", task.gates.l1.exit),
                ("L2", task.gates.l2.exit),
                ("L3", task.gates.l3.exit),
                ("L4", task.gates.l4.exit),
            ];

            for (name, exit) in gates {
                if exit != Some(0) {
                    return Err(GuardFailure::GateFailed {
                        gate: name.to_string(),
                        exit,
                    });
                }
            }

            // No error hash at count >= 2
            for err in &task.errors {
                if err.count >= 2 {
                    return Err(GuardFailure::DuplicateErrorThreshold {
                        hash: err.hash.clone(),
                        count: err.count,
                    });
                }
            }

            Ok(())
        }

        // Checker -> Human
        (TaskRole::Checker, TaskRole::Human) => {
            if target_status != TaskStatus::AwaitingHitl {
                return Err(GuardFailure::InvalidStatusTransition {
                    from_status: current_status,
                    to_status: target_status,
                });
            }
            Ok(())
        }

        // Human -> Completed / SentBack
        (TaskRole::Human, TaskRole::Human) => {
            if target_status != TaskStatus::Completed && target_status != TaskStatus::SentBack {
                return Err(GuardFailure::InvalidStatusTransition {
                    from_status: current_status,
                    to_status: target_status,
                });
            }
            Ok(())
        }

        // Human -> Planner (Send back to re-plan)
        (TaskRole::Human, TaskRole::Planner) => {
            if target_status != TaskStatus::SentBack && target_status != TaskStatus::Planned {
                return Err(GuardFailure::InvalidStatusTransition {
                    from_status: current_status,
                    to_status: target_status,
                });
            }
            Ok(())
        }

        // Non-standard transition
        (from, to) => Err(GuardFailure::InvalidRoleTransition {
            from_role: from,
            to_role: to,
        }),
    }
}
