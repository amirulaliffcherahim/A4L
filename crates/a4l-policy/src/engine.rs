use glob::Pattern;
use std::path::Path;
use thiserror::Error;

use crate::rule::{ActionRequest, PolicyConfig, PolicyDecision, Role, RuleDecision};

#[derive(Debug, Error)]
pub enum PolicyLoadError {
    #[error("io error reading policy: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml parse error: {0}")]
    Yaml(#[from] serde_yml::Error),
}

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    role: Role,
    policies: Vec<PolicyConfig>,
    test_fence_glob: String,
}

impl PolicyEngine {
    pub fn new(role: Role) -> Self {
        Self {
            role,
            policies: Vec::new(),
            test_fence_glob: "tests/**".to_string(),
        }
    }

    pub fn with_test_fence_glob(mut self, glob: impl Into<String>) -> Self {
        self.test_fence_glob = glob.into();
        self
    }

    pub fn set_role(&mut self, role: Role) {
        self.role = role;
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn add_policy(&mut self, policy: PolicyConfig) {
        self.policies.push(policy);
    }

    pub fn load_from_str(&mut self, yaml: &str) -> Result<(), PolicyLoadError> {
        let policy: PolicyConfig = serde_yml::from_str(yaml)?;
        self.add_policy(policy);
        Ok(())
    }

    pub fn load_from_dir(&mut self, dir: &Path) -> Result<usize, PolicyLoadError> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(0);
        }

        let mut loaded = 0;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                || path.extension().and_then(|s| s.to_str()) == Some("yml")
            {
                let content = std::fs::read_to_string(&path)?;
                let policy: PolicyConfig = serde_yml::from_str(&content)?;
                self.add_policy(policy);
                loaded += 1;
            }
        }
        Ok(loaded)
    }

    pub fn evaluate(&self, request: &ActionRequest) -> PolicyDecision {
        // 1. Immutable invariants
        match request {
            ActionRequest::FileWrite { path } | ActionRequest::FileEdit { path } => {
                let norm = normalize_path(path);

                // Policy tampering protection
                if norm.starts_with(".a4l/policies") || norm.starts_with("policies/") {
                    return PolicyDecision::Deny(
                        "modifying .a4l/policies is strictly denied during execution".to_string(),
                    );
                }

                // Role-based write restrictions
                match self.role {
                    Role::Planner => {
                        return PolicyDecision::Deny(
                            "Planner role is read-only; writes are denied".to_string(),
                        );
                    }
                    Role::Checker => {
                        return PolicyDecision::Deny(
                            "Checker role is read-only; writes are denied".to_string(),
                        );
                    }
                    Role::Maker => {
                        // Immutable test fence
                        if self.matches_test_fence(&norm) {
                            return PolicyDecision::Deny(format!(
                                "tests/** is immutable in Maker role: '{}'",
                                path
                            ));
                        }
                    }
                    Role::Human => {}
                }
            }
            ActionRequest::FileRead { .. } => {
                // Reads are generally safe unless blocked by policy below
            }
            ActionRequest::Git { op, .. } => {
                // Hard protection: Maker or Checker cannot push or force-clean
                if (self.role == Role::Maker || self.role == Role::Checker)
                    && (op == "push" || op == "clean")
                {
                    return PolicyDecision::Deny(format!(
                        "git {} is denied in {} role",
                        op, self.role
                    ));
                }
            }
            ActionRequest::Command { cmd } => {
                // Reject obvious shell jailbreaks or raw dangerous commands if role is not Human
                let trimmed = cmd.trim();
                if trimmed.starts_with("rm -rf /") || trimmed.starts_with("format ") {
                    return PolicyDecision::Deny("destructive command rejected".to_string());
                }
            }
        }

        // 2. Evaluate configured policy rules
        for policy in &self.policies {
            if let Some(dec) = self.evaluate_policy_rules(policy, request) {
                return dec;
            }
        }

        // 3. Fallback defaults
        match request {
            ActionRequest::FileRead { .. } => {
                // Standard reads default to allow
                PolicyDecision::Allow
            }
            ActionRequest::FileWrite { .. } | ActionRequest::FileEdit { .. } => {
                if self.role == Role::Maker {
                    // Maker writing to non-test files defaults to Allow
                    PolicyDecision::Allow
                } else if self.role == Role::Human {
                    PolicyDecision::Allow
                } else {
                    PolicyDecision::Ask(
                        "write operation outside maker/human requires approval".to_string(),
                    )
                }
            }
            ActionRequest::Command { .. } => {
                // Unknown commands default to Ask per specification
                PolicyDecision::Ask(
                    "unrecognized command pattern requires confirmation".to_string(),
                )
            }
            ActionRequest::Git { .. } => {
                PolicyDecision::Ask("git operation requires confirmation".to_string())
            }
        }
    }

    fn matches_test_fence(&self, path: &str) -> bool {
        if path.starts_with("tests/") || path.starts_with("test/") || path == "tests" {
            return true;
        }
        if let Ok(pattern) = Pattern::new(&self.test_fence_glob) {
            if pattern.matches(path) {
                return true;
            }
        }
        false
    }

    fn evaluate_policy_rules(
        &self,
        policy: &PolicyConfig,
        request: &ActionRequest,
    ) -> Option<PolicyDecision> {
        match request {
            ActionRequest::Command { cmd } => {
                let trimmed = cmd.trim();
                for rule in &policy.commands {
                    if self.matches_pattern(&rule.pattern, trimmed) {
                        return Some(self.convert_decision(&rule.decision, rule.reason.as_deref()));
                    }
                }
            }
            ActionRequest::FileRead { path } => {
                let norm = normalize_path(path);
                for rule in &policy.paths {
                    if self.matches_pattern(&rule.glob, &norm) {
                        if let Some(read_dec) = &rule.read {
                            return Some(self.convert_decision(read_dec, rule.reason.as_deref()));
                        }
                    }
                }
            }
            ActionRequest::FileWrite { path } | ActionRequest::FileEdit { path } => {
                let norm = normalize_path(path);
                for rule in &policy.paths {
                    if self.matches_pattern(&rule.glob, &norm) {
                        if let Some(write_dec) = &rule.write {
                            return Some(self.convert_decision(write_dec, rule.reason.as_deref()));
                        }
                    }
                }
            }
            ActionRequest::Git { op, .. } => {
                for rule in &policy.git {
                    if rule.op == "*" || rule.op.eq_ignore_ascii_case(op) {
                        return Some(self.convert_decision(&rule.decision, rule.reason.as_deref()));
                    }
                }
            }
        }
        None
    }

    fn matches_pattern(&self, pattern_str: &str, target: &str) -> bool {
        if pattern_str == "*" {
            return true;
        }
        if let Ok(pattern) = Pattern::new(pattern_str) {
            if pattern.matches(target) {
                return true;
            }
        }
        target.starts_with(pattern_str)
    }

    fn convert_decision(&self, decision: &RuleDecision, reason: Option<&str>) -> PolicyDecision {
        match decision {
            RuleDecision::Allow => PolicyDecision::Allow,
            RuleDecision::Ask => PolicyDecision::Ask(
                reason
                    .unwrap_or("policy rule requires confirmation")
                    .to_string(),
            ),
            RuleDecision::Deny => {
                PolicyDecision::Deny(reason.unwrap_or("policy rule denied action").to_string())
            }
        }
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}
