pub mod commands;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_all_templates_yaml_validity() {
        use a4l_core::assembler::ModelProfile;
        use a4l_core::provider::ProvidersFile;
        use a4l_policy::PolicyConfig;

        // Test policies
        let default_policy: PolicyConfig =
            serde_yml::from_str(include_str!("../../../templates/policies/default.yaml"))
                .expect("default.yaml valid");
        assert_eq!(default_policy.name, "default-policy");

        let test_lock_policy: PolicyConfig =
            serde_yml::from_str(include_str!("../../../templates/policies/test-lock.yaml"))
                .expect("test-lock.yaml valid");
        assert_eq!(test_lock_policy.name, "test-lock-policy");

        // Test providers
        let providers: ProvidersFile =
            serde_yml::from_str(include_str!("../../../templates/providers.yaml"))
                .expect("providers.yaml valid");
        assert_eq!(providers.default_provider, "openai-compatible");

        // Test profiles
        let _: ModelProfile =
            serde_yml::from_str(include_str!("../../../templates/profiles/frontier.yaml"))
                .expect("frontier.yaml valid");
        let _: ModelProfile =
            serde_yml::from_str(include_str!("../../../templates/profiles/standard.yaml"))
                .expect("standard.yaml valid");
        let _: ModelProfile =
            serde_yml::from_str(include_str!("../../../templates/profiles/small.yaml"))
                .expect("small.yaml valid");
    }

    #[test]
    fn test_init_scaffolding() {
        let dir = tempdir().unwrap();
        commands::init::run_init(dir.path(), false).expect("init should succeed");

        assert!(dir.path().join(".a4l").join("A4L.md").exists());
        assert!(dir.path().join(".a4l").join("init.sh").exists());
        assert!(dir.path().join(".a4l").join("tasks.json").exists());
        assert!(dir.path().join(".a4l").join("MEMORY.md").exists());
        assert!(dir
            .path()
            .join(".a4l")
            .join("policies")
            .join("default.yaml")
            .exists());
        assert!(dir
            .path()
            .join(".a4l")
            .join("profiles")
            .join("standard.yaml")
            .exists());
        assert!(dir
            .path()
            .join(".a4l")
            .join("roles")
            .join("planner.md")
            .exists());
        assert!(dir.path().join("docs").join("testing.md").exists());
        assert!(dir
            .path()
            .join(".a4l")
            .join("skills")
            .join("create-skill.md")
            .exists());
        assert!(dir
            .path()
            .join(".a4l")
            .join("skills")
            .join("setup-mcp.md")
            .exists());
        assert!(dir
            .path()
            .join(".a4l")
            .join("skills")
            .join("create-extension.md")
            .exists());
    }

    #[test]
    fn test_ext_cli_commands() {
        let dir = tempdir().unwrap();
        let a4l_home = dir.path().join(".a4l");

        // Add
        commands::ext::run_ext_add(&a4l_home, "my-tool", "github.com/a4l/my-tool").unwrap();

        // List
        commands::ext::run_ext_list(&a4l_home).unwrap();

        // Remove
        commands::ext::run_ext_remove(&a4l_home, "my-tool").unwrap();
    }

    #[test]
    fn test_memory_cli_commands() {
        let dir = tempdir().unwrap();
        commands::init::run_init(dir.path(), false).unwrap();

        // Show memory
        commands::memory::run_memory_show(dir.path()).unwrap();

        // Search memory (empty or no db)
        commands::memory::run_memory_search(dir.path(), "cannot find").unwrap();
    }
}
