pub mod buffer;
pub mod runner;

pub use buffer::RollingBuffer;
pub use runner::{PtyError, PtyExecutionResult, PtyRunner};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_pty_runner_executes_and_captures_output() {
        let dir = tempdir().unwrap();
        let history = dir.path().join(".a4l").join("history");
        let runner = PtyRunner::new(&history).with_buffer_sizes(50, 50);

        let res = runner
            .run_command(
                dir.path(),
                "T-0001",
                "echo hello from pty",
                Duration::from_secs(5),
            )
            .await
            .unwrap();

        assert_eq!(res.exit_code, 0);
        assert!(res.summary.contains("hello from pty"));
        assert!(dir.path().join(&res.log_path).exists());
    }

    #[tokio::test]
    async fn test_pty_runner_large_output_bounding() {
        let dir = tempdir().unwrap();
        let history = dir.path().join(".a4l").join("history");
        // Tight 20 bytes head and tail buffer
        let runner = PtyRunner::new(&history).with_buffer_sizes(20, 20);

        // Generate output with multiple lines
        #[cfg(target_os = "windows")]
        let cmd = "echo LINE1_HEAD_SECTION_START && echo LINE2_MIDDLE_SECTION_SHOULD_BE_ELIDED && echo LINE3_TAIL_SECTION_END";
        #[cfg(not(target_os = "windows"))]
        let cmd = "echo 'LINE1_HEAD_SECTION_START'; echo 'LINE2_MIDDLE_SECTION_SHOULD_BE_ELIDED'; echo 'LINE3_TAIL_SECTION_END'";

        let res = runner
            .run_command(dir.path(), "T-0002", cmd, Duration::from_secs(5))
            .await
            .unwrap();

        assert_eq!(res.exit_code, 0);
        assert!(res.is_truncated);
        assert!(res.summary.contains("[... <elided "));
        assert!(res.total_bytes > 40);

        // Verify full log on disk contains everything
        let full_log = std::fs::read_to_string(dir.path().join(&res.log_path)).unwrap();
        assert!(full_log.contains("LINE2_MIDDLE_SECTION_SHOULD_BE_ELIDED"));
    }
}
