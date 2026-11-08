use a4l_memory::{BoundedMemoryManager, EpisodicStore};
use colored::Colorize;
use std::path::Path;

pub fn run_memory_search(repo_root: &Path, symptom_query: &str) -> anyhow::Result<()> {
    let db_path = repo_root.join(".a4l").join("memory.db");
    if !db_path.exists() {
        println!(
            "{}",
            "No episodic memory database found at .a4l/memory.db.".yellow()
        );
        return Ok(());
    }

    let store = EpisodicStore::open(&db_path)?;
    println!(
        "{}",
        format!("Searching episodic memories for '{}'...", symptom_query).cyan()
    );
    let episodes = store.search_by_symptom(symptom_query)?;

    if episodes.is_empty() {
        println!("No matching failure bundles or task episodes found.");
        return Ok(());
    }

    println!(
        "{}",
        format!("Found {} relevant episode(s):", episodes.len())
            .bold()
            .green()
    );
    for ep in episodes {
        println!(
            "\n[{}] {} ({})",
            ep.task_id.bold(),
            ep.title,
            ep.kind.dimmed()
        );
        println!("  Symptoms:   {}", ep.symptoms.yellow());
        println!("  Resolution: {}", ep.resolution.green());
        println!("  Recorded:   {}", ep.created_at.dimmed());
    }

    Ok(())
}

pub fn run_memory_show(repo_root: &Path) -> anyhow::Result<()> {
    let mem_path = repo_root.join(".a4l").join("MEMORY.md");
    if !mem_path.exists() {
        println!(
            "{}",
            "No .a4l/MEMORY.md found. Run 'a4l init' first.".yellow()
        );
        return Ok(());
    }

    let manager = BoundedMemoryManager::new(&mem_path, 200);
    let content = manager.read_memory()?;
    println!(
        "{}",
        "=== BOUNDED PROJECT MEMORY (.a4l/MEMORY.md) ==="
            .bold()
            .cyan()
    );
    println!("{}", content);
    Ok(())
}
