mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "a4l")]
#[command(about = "A4L — Deterministic Human-in-the-Loop Agentic Harness", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Target project directory
    #[arg(short, long, global = true)]
    dir: Option<PathBuf>,

    /// Disable custom compiled extensions and run clean base binary
    #[arg(long, global = true)]
    no_extensions: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .a4l/ workspace in the project
    Init {
        /// Force overwrite existing template files
        #[arg(short, long)]
        force: bool,
    },

    /// Run diagnostics on system, providers, and workspace
    Doctor,

    /// Show current pipeline status and queue
    Status,

    /// Execute a task or watch the task queue
    Run {
        /// Target task ID (e.g. T-0001)
        #[arg(short, long)]
        task: Option<String>,

        /// Watch file for changes (e.g. tasks.json)
        #[arg(short, long)]
        watch: Option<String>,

        /// Run in headless mode (never prompt for interactive approval)
        #[arg(long)]
        headless: bool,

        /// Run using offline mock provider
        #[arg(long)]
        mock: bool,
    },

    /// Start the headless background daemon API
    Serve {
        /// Address to bind (e.g. 127.0.0.1:4040)
        #[arg(short, long)]
        bind: Option<String>,

        /// Authentication bearer token
        #[arg(short, long)]
        token: Option<String>,

        /// Allow remote channel to approve final merge (disabled by default)
        #[arg(long)]
        allow_remote_merge: bool,
    },

    /// Manage compiled A4L extensions
    Ext {
        #[command(subcommand)]
        sub: ExtSubcommands,
    },

    /// View or manage todos
    Todos {
        #[arg(short, long)]
        task: Option<String>,
        #[arg(default_value = "list")]
        action: String,
        text: Option<String>,
    },
}

#[derive(Subcommand)]
enum ExtSubcommands {
    /// List installed extensions
    List,
    /// Add an extension from path or git url
    Add { name: String, source: String },
    /// Remove an installed extension
    Remove { name: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let repo_root = cli.dir.unwrap_or_else(|| std::env::current_dir().unwrap());

    match cli.command {
        Some(Commands::Init { force }) => {
            commands::init::run_init(&repo_root, force)?;
        }
        Some(Commands::Doctor) => {
            commands::doctor::run_doctor(&repo_root);
        }
        Some(Commands::Status) => {
            commands::status::run_status(&repo_root);
        }
        Some(Commands::Run {
            task,
            watch,
            headless,
            mock,
        }) => {
            if let Some(task_id) = task {
                commands::run::run_task_loop(&repo_root, &task_id, headless, mock).await?;
            } else if let Some(watch_file) = watch {
                println!(
                    "Watching {} for task updates (press Ctrl+C to exit)...",
                    watch_file
                );
                tokio::signal::ctrl_c().await?;
            } else {
                commands::status::run_status(&repo_root);
            }
        }
        Some(Commands::Serve {
            bind,
            token,
            allow_remote_merge,
        }) => {
            commands::serve::run_serve(&repo_root, bind, token, allow_remote_merge).await?;
        }
        Some(Commands::Ext { sub }) => {
            let a4l_home = repo_root.join(".a4l");
            match sub {
                ExtSubcommands::List => commands::ext::run_ext_list(&a4l_home)?,
                ExtSubcommands::Add { name, source } => {
                    commands::ext::run_ext_add(&a4l_home, &name, &source)?
                }
                ExtSubcommands::Remove { name } => commands::ext::run_ext_remove(&a4l_home, &name)?,
            }
        }
        Some(Commands::Todos { task, action, text }) => {
            let policy_engine = a4l_policy::PolicyEngine::new(a4l_policy::Role::Maker);
            let ctx = a4l_core::tools::ToolContext {
                repo_root: &repo_root,
                active_task_id: task,
                policy_engine: &policy_engine,
                is_headless: false,
            };
            let args = a4l_core::tools::todo::TodoArgs {
                action,
                id: None,
                text,
                status: None,
            };
            match a4l_core::tools::todo::execute(&ctx, args) {
                Ok(res) => {
                    println!("{}", res.message);
                    for it in res.items {
                        println!("  [{}] {} ({})", it.id, it.text, it.status);
                    }
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        None => {
            commands::status::run_status(&repo_root);
        }
    }

    Ok(())
}
