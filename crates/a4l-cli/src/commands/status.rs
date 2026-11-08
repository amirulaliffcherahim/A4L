use a4l_core::state::{CircuitBreakerStatus, StateStore, TaskStatus, TodoStatus};
use colored::Colorize;
use std::path::Path;

pub fn run_status(repo_root: &Path) {
    let tasks_file = repo_root.join(".a4l").join("tasks.json");
    if !tasks_file.exists() {
        println!(
            "{}",
            "No .a4l/tasks.json found. Run 'a4l init' first.".yellow()
        );
        return;
    }

    let store = StateStore::new(tasks_file);
    let tasks = match store.load() {
        Ok(t) => t,
        Err(e) => {
            println!("{} Failed to read tasks: {}", "✗".red(), e);
            return;
        }
    };

    println!("{}", "=== A4L TASK PIPELINE STATUS ===".bold().cyan());
    if tasks.queue.is_empty() {
        println!("Queue is empty. No tasks currently registered.");
        return;
    }

    for task in &tasks.queue {
        let status_colored = match task.state.status {
            TaskStatus::Planned => "planned".cyan(),
            TaskStatus::InProgress => "in_progress".yellow(),
            TaskStatus::Review => "review".blue(),
            TaskStatus::Verifying => "verifying".magenta(),
            TaskStatus::AwaitingHitl => "awaiting_hitl".bold().yellow(),
            TaskStatus::Completed => "completed".bold().green(),
            TaskStatus::Failed => "failed".bold().red(),
            TaskStatus::SentBack => "sent_back".red(),
        };

        println!(
            "\n[{}] {} (attempt {})",
            task.id.bold(),
            task.title,
            task.state.attempt
        );
        println!(
            "  Role:   {} | Status: {}",
            task.state.role.to_string().bold(),
            status_colored
        );

        if let Some(bundle) = &task.state.context_bundle {
            println!("  Context Bundle: {}", bundle.dimmed());
        }

        // Todos
        if !task.todos.is_empty() {
            println!("  TODOs:");
            for todo in &task.todos {
                let badge = match todo.status {
                    TodoStatus::Pending => "[ ]".dimmed(),
                    TodoStatus::Done => "[✓ claim]".yellow(),
                    TodoStatus::Verified => "[✓ verified]".green(),
                };
                println!("    {} {} {}", badge, todo.id.dimmed(), todo.text);
            }
        }

        // Gates
        let g = &task.gates;
        println!("  Verification Gates (L1–L4):");
        print_gate("L1 Static", &g.l1);
        print_gate("L2 Unit", &g.l2);
        print_gate("L3 Smoke", &g.l3);
        print_gate("L4 E2E", &g.l4);

        // Breaker
        if task.circuit_breaker.status == CircuitBreakerStatus::Tripped {
            println!(
                "  {} Breaker TRIPPED: {:?} (branch: {:?})",
                "⚡".red(),
                task.circuit_breaker.reason,
                task.circuit_breaker.quarantine_branch
            );
        }
    }
}

fn print_gate(tier: &str, gate: &a4l_core::state::GateResult) {
    let exit_str = match gate.exit {
        Some(0) => "PASS (exit 0)".green(),
        Some(code) => format!("FAIL (exit {})", code).red(),
        None => "UNRUN".dimmed(),
    };
    println!(
        "    {:<10} {:<16} cmd: {}",
        tier,
        exit_str,
        gate.cmd.dimmed()
    );
}
