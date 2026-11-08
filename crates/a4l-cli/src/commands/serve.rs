use a4l_server::{DaemonServer, ServerConfig};
use colored::Colorize;
use std::path::Path;

pub async fn run_serve(
    repo_root: &Path,
    bind_addr: Option<String>,
    token: Option<String>,
    allow_remote_merge: bool,
) -> anyhow::Result<()> {
    let mut config = ServerConfig::default();
    if let Some(addr) = bind_addr {
        config.bind_addr = addr;
    }
    if let Some(tok) = token {
        config.auth_token = tok;
    }
    config.allow_remote_merge = allow_remote_merge;

    println!("{}", "Starting A4L Headless Daemon Server...".bold().cyan());
    println!("  Bind Address:       {}", config.bind_addr.green());
    println!("  Auth Token:         {}", config.auth_token.yellow());
    println!(
        "  Allow Remote Merge: {}",
        if allow_remote_merge {
            "ENABLED".red()
        } else {
            "DISABLED (default)".green()
        }
    );
    println!(
        "  Repo Root:          {}",
        repo_root.display().to_string().dimmed()
    );

    let _server = DaemonServer::new(config);

    println!(
        "{}",
        "\nDaemon running. Listening for events & client commands (Ctrl+C to stop)...".green()
    );
    tokio::signal::ctrl_c().await?;
    println!("\n{}", "Daemon stopped cleanly.".cyan());
    Ok(())
}
