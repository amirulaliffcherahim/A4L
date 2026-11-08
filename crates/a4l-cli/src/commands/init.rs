use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

pub fn run_init(target_dir: &Path, force: bool) -> Result<()> {
    println!("{}", "Scaffolding A4L workspace...".bold().cyan());

    let a4l_dir = target_dir.join(".a4l");
    let docs_dir = target_dir.join("docs");

    std::fs::create_dir_all(&a4l_dir)?;
    std::fs::create_dir_all(a4l_dir.join("history"))?;
    std::fs::create_dir_all(a4l_dir.join("handoffs"))?;
    std::fs::create_dir_all(a4l_dir.join("contexts"))?;
    std::fs::create_dir_all(a4l_dir.join("failures"))?;
    std::fs::create_dir_all(a4l_dir.join("verdicts"))?;
    std::fs::create_dir_all(a4l_dir.join("session"))?;
    std::fs::create_dir_all(a4l_dir.join("roles"))?;
    std::fs::create_dir_all(a4l_dir.join("policies"))?;
    std::fs::create_dir_all(a4l_dir.join("profiles"))?;
    std::fs::create_dir_all(a4l_dir.join("skills"))?;
    std::fs::create_dir_all(&docs_dir)?;

    write_template(
        &a4l_dir.join("A4L.md"),
        include_str!("../../../../templates/A4L.md"),
        force,
    )?;
    write_template(
        &a4l_dir.join("init.sh"),
        include_str!("../../../../templates/init.sh"),
        force,
    )?;
    write_template(
        &a4l_dir.join("providers.yaml"),
        include_str!("../../../../templates/providers.yaml"),
        force,
    )?;
    write_template(
        &a4l_dir.join("tasks.json"),
        include_str!("../../../../templates/tasks.json"),
        force,
    )?;
    write_template(
        &a4l_dir.join("progress.md"),
        include_str!("../../../../templates/progress.md"),
        force,
    )?;

    write_template(
        &docs_dir.join("testing.md"),
        include_str!("../../../../templates/docs/testing.md"),
        force,
    )?;

    write_template(
        &a4l_dir.join("policies").join("default.yaml"),
        include_str!("../../../../templates/policies/default.yaml"),
        force,
    )?;
    write_template(
        &a4l_dir.join("policies").join("test-lock.yaml"),
        include_str!("../../../../templates/policies/test-lock.yaml"),
        force,
    )?;

    write_template(
        &a4l_dir.join("profiles").join("frontier.yaml"),
        include_str!("../../../../templates/profiles/frontier.yaml"),
        force,
    )?;
    write_template(
        &a4l_dir.join("profiles").join("standard.yaml"),
        include_str!("../../../../templates/profiles/standard.yaml"),
        force,
    )?;
    write_template(
        &a4l_dir.join("profiles").join("small.yaml"),
        include_str!("../../../../templates/profiles/small.yaml"),
        force,
    )?;

    write_template(
        &a4l_dir.join("roles").join("planner.md"),
        include_str!("../../../../templates/roles/planner.md"),
        force,
    )?;
    write_template(
        &a4l_dir.join("roles").join("maker.md"),
        include_str!("../../../../templates/roles/maker.md"),
        force,
    )?;
    write_template(
        &a4l_dir.join("roles").join("checker.md"),
        include_str!("../../../../templates/roles/checker.md"),
        force,
    )?;
    write_template(
        &a4l_dir.join("roles").join("architect.md"),
        include_str!("../../../../templates/roles/architect.md"),
        force,
    )?;

    write_template(
        &a4l_dir.join("skills").join("meta-architect.md"),
        include_str!("../../../../templates/skills/meta-architect.md"),
        force,
    )?;

    println!(
        "{}",
        "✓ A4L workspace initialized successfully in .a4l/".green()
    );
    Ok(())
}

fn write_template(path: &Path, content: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        println!("  {} already exists (skipping)", path.display());
        return Ok(());
    }
    std::fs::write(path, content).with_context(|| format!("failed writing {}", path.display()))?;
    println!("  {} created", path.display());
    Ok(())
}
