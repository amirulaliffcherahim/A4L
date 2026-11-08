use a4l_ext::ExtensionManager;
use colored::Colorize;
use std::path::Path;

pub fn run_ext_list(a4l_home: &Path) -> anyhow::Result<()> {
    let manager = ExtensionManager::new(a4l_home);
    let list = manager.list()?;
    println!("{}", "=== INSTALLED A4L EXTENSIONS ===".bold().cyan());
    if list.is_empty() {
        println!("No extensions currently installed.");
        return Ok(());
    }

    for ext in list {
        println!(
            "- {} (source: {}, hash: {})",
            ext.name.bold(),
            ext.source.dimmed(),
            &ext.content_hash[..8]
        );
    }
    Ok(())
}

pub fn run_ext_add(a4l_home: &Path, name: &str, source: &str) -> anyhow::Result<()> {
    let manager = ExtensionManager::new(a4l_home);
    println!("Installing extension '{}' from '{}'...", name, source);
    let entry = manager.add(name, source)?;
    println!(
        "{} Extension '{}' installed successfully! Hash: {}",
        "✓".green(),
        entry.name,
        &entry.content_hash[..8]
    );
    Ok(())
}

pub fn run_ext_remove(a4l_home: &Path, name: &str) -> anyhow::Result<()> {
    let manager = ExtensionManager::new(a4l_home);
    manager.remove(name)?;
    println!("{} Extension '{}' removed.", "✓".green(), name);
    Ok(())
}
