pub mod engine;
pub mod fence;
pub mod rule;
pub mod stop_hook;

pub use engine::{PolicyEngine, PolicyLoadError};
pub use fence::{check_git_test_fence, revert_test_fence, FenceError, FenceStatus};
pub use rule::{
    ActionRequest, CommandRule, GitRule, PathRule, PolicyConfig, PolicyDecision, Role, RuleDecision,
};
pub use stop_hook::{verify_stop_hook, StopHookError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_write_restrictions() {
        // Planner is read-only
        let planner_engine = PolicyEngine::new(Role::Planner);
        let dec = planner_engine.evaluate(&ActionRequest::FileWrite {
            path: "src/main.rs".to_string(),
        });
        assert!(matches!(dec, PolicyDecision::Deny(_)));

        let read_dec = planner_engine.evaluate(&ActionRequest::FileRead {
            path: "src/main.rs".to_string(),
        });
        assert!(matches!(read_dec, PolicyDecision::Allow));

        // Checker is read-only
        let checker_engine = PolicyEngine::new(Role::Checker);
        let dec = checker_engine.evaluate(&ActionRequest::FileEdit {
            path: "src/lib.rs".to_string(),
        });
        assert!(matches!(dec, PolicyDecision::Deny(_)));
    }

    #[test]
    fn test_maker_test_fence() {
        let maker_engine = PolicyEngine::new(Role::Maker);

        // Maker can write src
        let src_dec = maker_engine.evaluate(&ActionRequest::FileWrite {
            path: "src/lib.rs".to_string(),
        });
        assert!(matches!(src_dec, PolicyDecision::Allow));

        // Maker cannot write tests
        let test_dec = maker_engine.evaluate(&ActionRequest::FileWrite {
            path: "tests/suite.rs".to_string(),
        });
        assert!(matches!(test_dec, PolicyDecision::Deny(_)));

        // Maker cannot edit tests
        let edit_dec = maker_engine.evaluate(&ActionRequest::FileEdit {
            path: "tests/unit/test1.rs".to_string(),
        });
        assert!(matches!(edit_dec, PolicyDecision::Deny(_)));
    }

    #[test]
    fn test_policy_protection() {
        let maker_engine = PolicyEngine::new(Role::Maker);
        let dec = maker_engine.evaluate(&ActionRequest::FileWrite {
            path: ".a4l/policies/default.yaml".to_string(),
        });
        assert!(matches!(dec, PolicyDecision::Deny(_)));
    }

    #[test]
    fn test_table_driven_rules() {
        let yaml_config = r#"
name: standard-rules
commands:
  - pattern: "cargo test*"
    decision: allow
  - pattern: "rm -rf*"
    decision: deny
    reason: "no rm -rf allowed"
  - pattern: "npm install*"
    decision: ask
    reason: "installing packages requires confirmation"
paths:
  - glob: "secret/**"
    read: deny
    write: deny
git:
  - op: "status"
    decision: allow
  - op: "commit"
    decision: ask
"#;

        let mut engine = PolicyEngine::new(Role::Maker);
        engine.load_from_str(yaml_config).expect("valid yaml");

        struct TestCase {
            request: ActionRequest,
            expected: &'static str,
        }

        let cases = vec![
            TestCase {
                request: ActionRequest::Command {
                    cmd: "cargo test --all".to_string(),
                },
                expected: "allow",
            },
            TestCase {
                request: ActionRequest::Command {
                    cmd: "rm -rf /tmp".to_string(),
                },
                expected: "deny",
            },
            TestCase {
                request: ActionRequest::Command {
                    cmd: "npm install lodash".to_string(),
                },
                expected: "ask",
            },
            TestCase {
                request: ActionRequest::Command {
                    cmd: "python script.py".to_string(),
                },
                expected: "ask", // default unknown command is ask
            },
            TestCase {
                request: ActionRequest::FileRead {
                    path: "secret/key.pem".to_string(),
                },
                expected: "deny",
            },
            TestCase {
                request: ActionRequest::Git {
                    op: "status".to_string(),
                    args: vec![],
                },
                expected: "allow",
            },
            TestCase {
                request: ActionRequest::Git {
                    op: "commit".to_string(),
                    args: vec!["-m".to_string(), "feat".to_string()],
                },
                expected: "ask",
            },
        ];

        for case in cases {
            let res = engine.evaluate(&case.request);
            match case.expected {
                "allow" => assert!(res.is_allowed(), "expected allow for {:?}", case.request),
                "ask" => assert!(res.is_ask(), "expected ask for {:?}", case.request),
                "deny" => assert!(res.is_deny(), "expected deny for {:?}", case.request),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_stop_hook_verification() {
        let all_green = vec![
            ("L1", Some(0)),
            ("L2", Some(0)),
            ("L3", Some(0)),
            ("L4", Some(0)),
        ];
        assert_eq!(verify_stop_hook(all_green), Ok(()));

        let one_failed = vec![
            ("L1", Some(0)),
            ("L2", Some(101)),
            ("L3", Some(0)),
            ("L4", Some(0)),
        ];
        assert_eq!(
            verify_stop_hook(one_failed),
            Err(StopHookError::GateFailed {
                gate: "L2".to_string(),
                exit_code: 101
            })
        );

        let one_missing = vec![("L1", Some(0)), ("L2", None)];
        assert_eq!(
            verify_stop_hook(one_missing),
            Err(StopHookError::GateDidNotRun {
                gate: "L2".to_string()
            })
        );
    }
}
