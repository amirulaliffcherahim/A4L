use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Planner,
    Maker,
    Checker,
    Human,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Planner => write!(f, "planner"),
            Role::Maker => write!(f, "maker"),
            Role::Checker => write!(f, "checker"),
            Role::Human => write!(f, "human"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "reason", rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    Ask(String),
    Deny(String),
}

impl PolicyDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PolicyDecision::Allow)
    }

    pub fn is_deny(&self) -> bool {
        matches!(self, PolicyDecision::Deny(_))
    }

    pub fn is_ask(&self) -> bool {
        matches!(self, PolicyDecision::Ask(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ActionRequest {
    FileRead {
        path: String,
    },
    FileWrite {
        path: String,
    },
    FileEdit {
        path: String,
    },
    Command {
        cmd: String,
    },
    Git {
        op: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleDecision {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRule {
    pub pattern: String,
    pub decision: RuleDecision,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRule {
    pub glob: String,
    pub read: Option<RuleDecision>,
    pub write: Option<RuleDecision>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRule {
    pub op: String,
    pub decision: RuleDecision,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub commands: Vec<CommandRule>,
    #[serde(default)]
    pub paths: Vec<PathRule>,
    #[serde(default)]
    pub git: Vec<GitRule>,
}
