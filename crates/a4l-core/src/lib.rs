pub mod assembler;
pub mod circuit_breaker;
pub mod error_hash;
pub mod gate_runner;
pub mod guards;
pub mod handoff;
pub mod ledger;
pub mod provider;
pub mod schemas;
pub mod state;
pub mod tools;

pub use assembler::{Assembler, ModelProfile};
pub use circuit_breaker::{CircuitBreakerController, FailureBundle};
pub use error_hash::{compute_error_hash, normalize_error};
pub use gate_runner::GateRunner;
pub use guards::{can_advance, GuardFailure};
pub use handoff::{HandoffNote, HandoffPacket};
pub use ledger::{LayerTokens, LedgerEntry, TokenLedger};
pub use provider::{
    ChatMessage, LlmProvider, MockProvider, OpenAiProvider, ProviderConfig, ProvidersFile,
};
pub use state::{
    AcceptanceCriteria, CircuitBreaker, CircuitBreakerStatus, ErrorRecord, GateResult, Gates,
    StateStore, Task, TaskRole, TaskState, TaskStatus, TasksJson, TodoItem, TodoStatus, TripReason,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    fn make_test_task() -> Task {
        Task {
            id: "T-0001".to_string(),
            title: "Test Task".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            state: TaskState {
                status: TaskStatus::Planned,
                role: TaskRole::Planner,
                attempt: 1,
                context_bundle: Some(".a4l/contexts/T-0001-attempt1.md".to_string()),
            },
            todos: vec![TodoItem {
                id: "T-0001.1".to_string(),
                text: "Build handler".to_string(),
                status: TodoStatus::Pending,
                verified_at: None,
            }],
            acceptance_criteria: vec![AcceptanceCriteria {
                id: "AC-1".to_string(),
                check: "200 OK".to_string(),
                status: "unverified".to_string(),
            }],
            gates: Gates {
                l1: GateResult {
                    cmd: "cargo check".to_string(),
                    exit: None,
                    log: None,
                    ran_at: None,
                },
                l2: GateResult {
                    cmd: "cargo test".to_string(),
                    exit: None,
                    log: None,
                    ran_at: None,
                },
                l3: GateResult {
                    cmd: "smoke.sh".to_string(),
                    exit: None,
                    log: None,
                    ran_at: None,
                },
                l4: GateResult {
                    cmd: "e2e.sh".to_string(),
                    exit: None,
                    log: None,
                    ran_at: None,
                },
            },
            errors: Vec::new(),
            circuit_breaker: CircuitBreaker::default(),
        }
    }

    #[test]
    fn test_planner_to_maker_guard() {
        let mut task = make_test_task();

        // Valid transition
        assert!(can_advance(&task, TaskRole::Maker, TaskStatus::InProgress).is_ok());

        // Fail if context bundle missing
        task.state.context_bundle = None;
        assert_eq!(
            can_advance(&task, TaskRole::Maker, TaskStatus::InProgress),
            Err(GuardFailure::MissingContextBundle {
                task_id: task.id.clone()
            })
        );

        // Fail if todos empty
        task.state.context_bundle = Some("context.md".to_string());
        task.todos.clear();
        assert_eq!(
            can_advance(&task, TaskRole::Maker, TaskStatus::InProgress),
            Err(GuardFailure::EmptyTodos {
                task_id: task.id.clone()
            })
        );
    }

    #[test]
    fn test_maker_to_checker_guard() {
        let mut task = make_test_task();
        task.state.role = TaskRole::Maker;
        task.state.status = TaskStatus::InProgress;

        // Fails because todos are pending
        assert!(can_advance(&task, TaskRole::Checker, TaskStatus::Review).is_err());

        // Set todo to done (model claim) -> still must fail!
        task.todos[0].status = TodoStatus::Done;
        assert!(can_advance(&task, TaskRole::Checker, TaskStatus::Review).is_err());

        // Set todo to verified
        task.todos[0].status = TodoStatus::Verified;

        // Still fails because gates are not exit 0
        assert!(can_advance(&task, TaskRole::Checker, TaskStatus::Review).is_err());

        // Make all gates green
        task.gates.l1.exit = Some(0);
        task.gates.l2.exit = Some(0);
        task.gates.l3.exit = Some(0);
        task.gates.l4.exit = Some(0);

        // Now passes!
        assert!(can_advance(&task, TaskRole::Checker, TaskStatus::Review).is_ok());

        // But fails if gate L2 failed
        task.gates.l2.exit = Some(1);
        assert_eq!(
            can_advance(&task, TaskRole::Checker, TaskStatus::Review),
            Err(GuardFailure::GateFailed {
                gate: "L2".to_string(),
                exit: Some(1)
            })
        );
    }

    #[test]
    fn test_checker_cannot_mark_completed() {
        let mut task = make_test_task();
        task.state.role = TaskRole::Checker;
        task.state.status = TaskStatus::Review;

        // Checker trying to mark completed must be rejected
        let res = can_advance(&task, TaskRole::Checker, TaskStatus::Completed);
        assert!(res.is_err());

        // Checker can only advance to Human (AwaitingHitl)
        assert!(can_advance(&task, TaskRole::Human, TaskStatus::AwaitingHitl).is_ok());
    }

    #[test]
    fn test_human_executive_completion_and_send_back() {
        let mut task = make_test_task();
        task.state.role = TaskRole::Human;
        task.state.status = TaskStatus::AwaitingHitl;

        // Human approves
        assert!(can_advance(&task, TaskRole::Human, TaskStatus::Completed).is_ok());

        // Human sends back to Planner
        assert!(can_advance(&task, TaskRole::Planner, TaskStatus::SentBack).is_ok());

        // Maker cannot jump directly to completed
        task.state.role = TaskRole::Maker;
        assert!(can_advance(&task, TaskRole::Maker, TaskStatus::Completed).is_err());
    }

    #[test]
    fn test_circuit_breaker_duplicate_error_and_trip() {
        let dir = tempdir().unwrap();
        let controller = CircuitBreakerController::new(dir.path(), 3);
        let mut task = make_test_task();
        task.state.role = TaskRole::Maker;
        task.state.status = TaskStatus::InProgress;

        // No trip initially
        assert_eq!(controller.check_trip(&task, false), None);

        // Add duplicate error hash with count = 2
        task.errors.push(ErrorRecord {
            hash: "abc123hash".to_string(),
            count: 2,
            last_seen: Utc::now(),
        });

        assert_eq!(
            controller.check_trip(&task, false),
            Some(TripReason::DuplicateError)
        );

        // Trip and rebase
        let bundle = controller
            .trip_and_rebase(&mut task, TripReason::DuplicateError)
            .unwrap();
        assert_eq!(bundle.trip_reason, TripReason::DuplicateError);
        assert_eq!(task.state.role, TaskRole::Planner);
        assert_eq!(task.state.status, TaskStatus::Planned);
        assert_eq!(task.state.attempt, 2);
        assert_eq!(task.circuit_breaker.status, CircuitBreakerStatus::Tripped);
    }

    #[test]
    fn test_circuit_breaker_third_attempt_ceiling() {
        let dir = tempdir().unwrap();
        let controller = CircuitBreakerController::new(dir.path(), 3);
        let mut task = make_test_task();
        task.state.role = TaskRole::Maker;
        task.state.status = TaskStatus::InProgress;
        task.state.attempt = 3;

        // Attempt 3 failing Maker gate trips breaker
        assert_eq!(
            controller.check_trip(&task, true),
            Some(TripReason::MaxAttempts)
        );

        // Trip and rebase routes to Planner with attempt 4
        let bundle = controller
            .trip_and_rebase(&mut task, TripReason::MaxAttempts)
            .unwrap();
        assert_eq!(bundle.trip_reason, TripReason::MaxAttempts);
        assert_eq!(task.state.role, TaskRole::Planner);
        assert_eq!(task.state.status, TaskStatus::Planned);
        assert_eq!(task.state.attempt, 4);
    }

    #[test]
    fn test_handoff_model_switch_and_resume_equivalence() {
        let task = make_test_task();
        let note = HandoffNote {
            goal: "Deliver metrics endpoint".to_string(),
            decisions: vec![crate::handoff::Decision {
                what: "Use Prometheus format".to_string(),
                why: "Standard ops requirement".to_string(),
            }],
            next: "Verify e2e curl".to_string(),
            pitfalls: vec![],
            open_questions: vec![],
        };

        // Outgoing Model A (e.g. gpt-4o) builds packet
        let packet = HandoffPacket::new_from_task(&task, 1, "model_switch", "gpt-4o", note, false);

        // Incoming Model B (e.g. small local model) profile
        let small_profile = ModelProfile {
            instruction_depth: "strict".to_string(),
            tail_turns: 2,
            note_max_tokens: 200,
            tool_output_limit: 1000,
        };

        let assembler = Assembler::new("# Kernel A4L", "Strict profile instructions")
            .with_profile(small_profile);

        let messages = assembler.assemble(
            "# Maker Role",
            Some(&task.id),
            "maker",
            &["meta-architect".to_string()],
            Some(&packet),
            &[],
            None,
        );

        // System prompt contains Layer 1-6
        assert_eq!(messages.len(), 1);
        let sys_content = messages[0].content.as_ref().unwrap();
        assert!(sys_content.contains("# Kernel A4L"));
        assert!(sys_content.contains("Strict profile instructions"));
        assert!(sys_content.contains("# Maker Role"));
        assert!(sys_content.contains("[A4L STATE]"));
        assert!(sys_content.contains("[HANDOFF PACKET]"));
        assert!(sys_content.contains("Deliver metrics endpoint"));
    }

    #[test]
    fn test_handoff_facts_integrity_and_cap() {
        let task = make_test_task();
        let mut note = HandoffNote {
            goal: "Do X".to_string(),
            decisions: vec![],
            next: "Next action step".to_string(),
            pitfalls: vec!["Too much complexity".to_string()],
            open_questions: vec![],
        };

        let mut packet = HandoffPacket::new_from_task(
            &task,
            1,
            "role_transition",
            "mock-model",
            note.clone(),
            false,
        );

        // Facts integrity: facts come from task state
        assert_eq!(packet.facts.status, "planned");
        assert_eq!(packet.facts.todos.len(), 1);

        // Cap enforcement
        note.next = "a".repeat(1000);
        packet.note = note;
        packet.enforce_note_cap(50);
        assert!(packet.note.next.len() <= 100);
    }

    #[test]
    fn test_state_store_atomic_roundtrip() {
        let dir = tempdir().unwrap();
        let tasks_file = dir.path().join(".a4l").join("tasks.json");
        let store = StateStore::new(&tasks_file);

        let mut tasks = TasksJson::default();
        let task = make_test_task();
        tasks.queue.push(task);

        store.save(&tasks).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.queue.len(), 1);
        assert_eq!(loaded.queue[0].id, "T-0001");

        // Update task
        let updated = store
            .update_task("T-0001", |t| {
                t.title = "Updated Title".to_string();
                Ok(())
            })
            .unwrap();

        assert_eq!(updated.title, "Updated Title");
    }
}
