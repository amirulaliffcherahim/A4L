pub mod commands;

pub use commands::{
    advance_task, approve_hitl, get_diff, get_gate_proofs, get_tasks, list_skills, search_memory,
    send_back_hitl, DesktopError, SkillSummary,
};

#[cfg(test)]
mod tests {
    use super::*;
    use a4l_core::state::{
        AcceptanceCriteria, CircuitBreaker, GateResult, Gates, StateStore, Task, TaskRole,
        TaskState, TaskStatus, TasksJson, TodoItem, TodoStatus,
    };
    use chrono::Utc;
    use tempfile::tempdir;

    fn make_test_task() -> Task {
        Task {
            id: "T-0001".to_string(),
            title: "Build health check".to_string(),
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
                text: "Route handler".to_string(),
                status: TodoStatus::Verified,
                verified_at: Some(Utc::now()),
            }],
            acceptance_criteria: vec![AcceptanceCriteria {
                id: "AC-1".to_string(),
                check: "returns 200".to_string(),
                status: "verified".to_string(),
            }],
            gates: Gates {
                l1: GateResult {
                    cmd: "cargo check".to_string(),
                    exit: Some(0),
                    log: None,
                    ran_at: Some(Utc::now()),
                },
                l2: GateResult {
                    cmd: "cargo test".to_string(),
                    exit: Some(0),
                    log: None,
                    ran_at: Some(Utc::now()),
                },
                l3: GateResult {
                    cmd: "smoke.sh".to_string(),
                    exit: Some(0),
                    log: None,
                    ran_at: Some(Utc::now()),
                },
                l4: GateResult {
                    cmd: "e2e.sh".to_string(),
                    exit: Some(0),
                    log: None,
                    ran_at: Some(Utc::now()),
                },
            },
            errors: Vec::new(),
            circuit_breaker: CircuitBreaker::default(),
        }
    }

    #[test]
    fn test_desktop_task_lifecycle_end_to_end() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path().to_string_lossy().to_string();
        let a4l_dir = dir.path().join(".a4l");
        std::fs::create_dir_all(&a4l_dir).unwrap();
        let tasks_file = a4l_dir.join("tasks.json");
        let store = StateStore::new(&tasks_file);

        let mut tasks = TasksJson::default();
        let task = make_test_task();
        tasks.queue.push(task);
        store.save(&tasks).unwrap();

        // 1. Get tasks
        let loaded = get_tasks(repo_root.clone()).unwrap();
        assert_eq!(loaded.queue.len(), 1);

        // 2. Advance Planner -> Maker
        let t1 = advance_task(
            repo_root.clone(),
            "T-0001".to_string(),
            TaskRole::Maker,
            TaskStatus::InProgress,
        )
        .unwrap();
        assert_eq!(t1.state.role, TaskRole::Maker);

        // 3. Advance Maker -> Checker (all gates are exit 0, todos verified)
        let t2 = advance_task(
            repo_root.clone(),
            "T-0001".to_string(),
            TaskRole::Checker,
            TaskStatus::Review,
        )
        .unwrap();
        assert_eq!(t2.state.role, TaskRole::Checker);

        // 4. Advance Checker -> Human (Awaiting HITL)
        let t3 = advance_task(
            repo_root.clone(),
            "T-0001".to_string(),
            TaskRole::Human,
            TaskStatus::AwaitingHitl,
        )
        .unwrap();
        assert_eq!(t3.state.role, TaskRole::Human);
        assert_eq!(t3.state.status, TaskStatus::AwaitingHitl);

        // 5. Desktop HITL Approve & Complete
        let completed = approve_hitl(repo_root, "T-0001".to_string()).unwrap();
        assert_eq!(completed.state.status, TaskStatus::Completed);
    }

    #[test]
    fn test_desktop_hitl_send_back() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path().to_string_lossy().to_string();
        let a4l_dir = dir.path().join(".a4l");
        let ctx_dir = a4l_dir.join("contexts");
        std::fs::create_dir_all(&ctx_dir).unwrap();
        let bundle_file = ctx_dir.join("T-0001-attempt1.md");
        std::fs::write(&bundle_file, "# Context Bundle\nInitial notes.\n").unwrap();

        let tasks_file = a4l_dir.join("tasks.json");
        let store = StateStore::new(&tasks_file);

        let mut tasks = TasksJson::default();
        let mut task = make_test_task();
        task.state.role = TaskRole::Human;
        task.state.status = TaskStatus::AwaitingHitl;
        tasks.queue.push(task);
        store.save(&tasks).unwrap();

        // Send back with notes
        let sent_back = send_back_hitl(
            repo_root,
            "T-0001".to_string(),
            "Please add Prometheus headers".to_string(),
        )
        .unwrap();
        assert_eq!(sent_back.state.role, TaskRole::Planner);
        assert_eq!(sent_back.state.status, TaskStatus::SentBack);
        assert_eq!(sent_back.state.attempt, 2);

        // Verify notes were appended to context bundle
        let updated_notes = std::fs::read_to_string(bundle_file).unwrap();
        assert!(updated_notes.contains("Please add Prometheus headers"));
    }

    #[test]
    fn test_desktop_list_skills_and_memory_search() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path().to_string_lossy().to_string();
        let skills_dir = dir.path().join(".a4l").join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("test-skill.md"),
            "# Test Skill\nDescription here\n",
        )
        .unwrap();

        let skills = list_skills(repo_root.clone()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");

        // Episodic memory
        let db_path = dir.path().join(".a4l").join("memory.db");
        let store = a4l_memory::EpisodicStore::open(&db_path).unwrap();
        store
            .insert_episode(
                "failure_bundle",
                "T-0001",
                "Compile error",
                "cannot find type `Foo`",
                "Fix import",
            )
            .unwrap();

        let results = search_memory(repo_root, "cannot find".to_string()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].task_id, "T-0001");
    }
}
