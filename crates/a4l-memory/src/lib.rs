pub mod bounded;
pub mod episodic;
pub mod sidechain;

pub use bounded::{BoundedMemoryManager, MemoryError};
pub use episodic::{Episode, EpisodicError, EpisodicStore};
pub use sidechain::{SidechainEntry, SidechainError, SidechainLogger};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_bounded_memory_manager_cap() {
        let dir = tempdir().unwrap();
        let mem_file = dir.path().join("MEMORY.md");
        let manager = BoundedMemoryManager::new(&mem_file, 5);

        manager
            .append_entry("# Project Memory\nRule 1: Always verify\nRule 2: Read before edit")
            .unwrap();
        let content = manager.read_memory().unwrap();
        assert!(content.contains("Rule 1"));

        // Overflow cap
        manager
            .append_entry(
                "Rule 3: Keep lean\nRule 4: Stop hooks\nRule 5: No loops\nRule 6: Human HITL",
            )
            .unwrap();
        let updated = manager.read_memory().unwrap();
        let line_count = updated.lines().count();
        assert!(line_count <= 6); // header + capped entries
        assert!(updated.contains("Rule 6"));
    }

    #[test]
    fn test_episodic_store_symptom_search() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("memory.db");
        let store = EpisodicStore::open(&db_path).unwrap();

        // Insert failure bundle episode
        store
            .insert_episode(
                "failure_bundle",
                "T-0001",
                "Compiler error in route handler",
                "error[E0425]: cannot find value `foo` in this scope",
                "Import foo from crate::metrics",
            )
            .unwrap();

        store
            .insert_episode(
                "task_summary",
                "T-0002",
                "Add health check endpoint",
                "no error",
                "curl /health returns 200",
            )
            .unwrap();

        // Search by symptom keyword "cannot find value"
        let matches = store.search_by_symptom("cannot find value").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].task_id, "T-0001");
        assert!(matches[0].symptoms.contains("cannot find value `foo`"));
    }

    #[test]
    fn test_sidechain_logger() {
        let dir = tempdir().unwrap();
        let logger = SidechainLogger::new(dir.path());

        let rel = logger
            .log_event(
                "T-0001",
                Some("sub-1"),
                "read",
                serde_json::json!({"path": "src/main.rs"}),
            )
            .unwrap();

        let log_file = dir.path().join(rel);
        assert!(log_file.exists());
        let content = std::fs::read_to_string(log_file).unwrap();
        assert!(content.contains("\"tool_name\":\"read\""));
    }
}
