use a4l_core::provider::ProvidersFile;
use a4l_core::state::StateStore;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub fn run_doctor(repo_root: &Path) {
    println!(
        "{}",
        "Running A4L Doctor system diagnostics...\n".bold().cyan()
    );

    // 1. Git
    print!("Checking git binary... ");
    match Command::new("git").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("{} ({})", "✓ OK".green(), ver);
        }
        _ => println!("{}", "✗ Missing or non-functional git".red()),
    }

    // Git repository check
    print!("Checking git repository status... ");
    match Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
    {
        Ok(out) if out.status.success() => println!("{}", "✓ OK (git repo confirmed)".green()),
        _ => println!(
            "{}",
            "⚠ Not inside a git repository (test fence requires git)".yellow()
        ),
    }

    // 2. Rust / Cargo
    print!("Checking cargo / rustc toolchain... ");
    match Command::new("cargo").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("{} ({})", "✓ OK".green(), ver);
        }
        _ => println!(
            "{}",
            "⚠ Cargo missing (needed for custom Rust extensions)".yellow()
        ),
    }

    // 3. Node / Bun runtime (optional, warning only)
    print!("Checking JavaScript runtime (Node/Bun)... ");
    let node_found = Command::new("node").arg("--version").output().is_ok();
    let bun_found = Command::new("bun").arg("--version").output().is_ok();
    if node_found || bun_found {
        println!("{}", "✓ OK".green());
    } else {
        println!(
            "{}",
            "ℹ None detected (optional, only for TS tooling)".dimmed()
        );
    }

    // 4. A4L Workspace Check
    print!("Checking .a4l/ workspace structure... ");
    let a4l_dir = repo_root.join(".a4l");
    if a4l_dir.exists() && a4l_dir.join("tasks.json").exists() {
        println!("{}", "✓ OK (.a4l/ initialized)".green());
    } else {
        println!(
            "{}",
            "⚠ Missing .a4l/ workspace. Run 'a4l init' first.".yellow()
        );
    }

    // 5. Providers configuration
    print!("Checking providers configuration... ");
    let prov_path = a4l_dir.join("providers.yaml");
    if prov_path.exists() {
        match ProvidersFile::load_from_path(&prov_path) {
            Ok(file) => {
                println!("{} (default: {})", "✓ OK".green(), file.default_provider);
                if let Some(def) = file.providers.get(&file.default_provider) {
                    if let Some(env_var) = &def.api_key_env {
                        if std::env::var(env_var).is_ok() {
                            println!("  {} Env var {} is set.", "✓".green(), env_var);
                        } else {
                            println!("  {} Env var {} is not set.", "⚠".yellow(), env_var);
                        }
                    }
                }
            }
            Err(e) => println!("{} ({})", "✗ Invalid providers.yaml".red(), e),
        }
    } else {
        println!(
            "{}",
            "ℹ No providers.yaml found (will use defaults or mock)".dimmed()
        );
    }

    // 6. MCP Configuration check
    print!("Checking Model Context Protocol (MCP) servers... ");
    let mcp_path = a4l_dir.join("mcp.json");
    if mcp_path.exists() {
        match a4l_mcp::McpConfigFile::load_from_path(&mcp_path) {
            Ok(cfg) => println!(
                "{} ({} servers configured)",
                "✓ OK".green(),
                cfg.mcp_servers.len()
            ),
            Err(e) => println!("{} ({})", "✗ Invalid mcp.json".red(), e),
        }
    } else {
        println!("{}", "ℹ No .a4l/mcp.json found (optional)".dimmed());
    }

    // 7. Tasks queue state
    let tasks_path = a4l_dir.join("tasks.json");
    if tasks_path.exists() {
        let store = StateStore::new(&tasks_path);
        match store.load() {
            Ok(tasks) => println!(
                "{} ({} tasks in queue)",
                "✓ Tasks store valid".green(),
                tasks.queue.len()
            ),
            Err(e) => println!("{} ({})", "✗ Failed to parse tasks.json".red(), e),
        }
    }

    println!("\n{}", "Diagnostics complete.".cyan());
}
