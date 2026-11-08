#[derive(Debug, PartialEq, Eq)]
pub enum LauncherTarget {
    CustomBinary,
    BaseBinary,
}

pub fn select_launcher_target(
    no_extensions_flag: bool,
    custom_bin_exists: bool,
    recent_crash_count: u32,
) -> LauncherTarget {
    // 1. Explicit --no-extensions flag always runs base
    if no_extensions_flag {
        return LauncherTarget::BaseBinary;
    }

    // 2. If crash count >= 2, fall back to base for safety
    if recent_crash_count >= 2 {
        return LauncherTarget::BaseBinary;
    }

    // 3. Run custom binary if compiled and available
    if custom_bin_exists {
        LauncherTarget::CustomBinary
    } else {
        LauncherTarget::BaseBinary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launcher_decision_matrix() {
        // Normal with custom build
        assert_eq!(
            select_launcher_target(false, true, 0),
            LauncherTarget::CustomBinary
        );

        // --no-extensions flag overrides custom
        assert_eq!(
            select_launcher_target(true, true, 0),
            LauncherTarget::BaseBinary
        );

        // Crash fallback (2 crashes)
        assert_eq!(
            select_launcher_target(false, true, 2),
            LauncherTarget::BaseBinary
        );

        // No custom build exists
        assert_eq!(
            select_launcher_target(false, false, 0),
            LauncherTarget::BaseBinary
        );
    }
}
