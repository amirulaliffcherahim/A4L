pub mod launcher;
pub mod manager;
pub mod manifest;

pub use launcher::{select_launcher_target, LauncherTarget};
pub use manager::{ExtensionManager, ExtensionManagerError};
pub use manifest::{ExtensionEntry, ExtensionsManifest};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extension_manager_add_list_remove() {
        let dir = tempdir().unwrap();
        let manager = ExtensionManager::new(dir.path());

        assert!(manager.list().unwrap().is_empty());

        let entry = manager
            .add("anthropic-provider", "github.com/a4l/anthropic")
            .unwrap();
        assert_eq!(entry.name, "anthropic-provider");

        let list = manager.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "anthropic-provider");

        // Remove
        manager.remove("anthropic-provider").unwrap();
        assert!(manager.list().unwrap().is_empty());
    }
}
