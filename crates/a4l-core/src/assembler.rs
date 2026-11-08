use serde::{Deserialize, Serialize};

use crate::handoff::HandoffPacket;
use crate::provider::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    #[serde(default = "default_instruction_depth")]
    pub instruction_depth: String, // "lite" | "standard" | "strict"
    #[serde(default = "default_tail_turns")]
    pub tail_turns: usize,
    #[serde(default = "default_note_max_tokens")]
    pub note_max_tokens: usize,
    #[serde(default = "default_tool_output_limit")]
    pub tool_output_limit: usize,
}

fn default_instruction_depth() -> String {
    "standard".to_string()
}
fn default_tail_turns() -> usize {
    4
}
fn default_note_max_tokens() -> usize {
    300
}
fn default_tool_output_limit() -> usize {
    2000
}

impl Default for ModelProfile {
    fn default() -> Self {
        Self {
            instruction_depth: default_instruction_depth(),
            tail_turns: default_tail_turns(),
            note_max_tokens: default_note_max_tokens(),
            tool_output_limit: default_tool_output_limit(),
        }
    }
}

pub struct Assembler {
    pub kernel_text: String,  // Layer 1 (A4L.md)
    pub profile_text: String, // Layer 3
    pub profile: ModelProfile,
}

impl Assembler {
    pub fn new(kernel_text: impl Into<String>, profile_text: impl Into<String>) -> Self {
        Self {
            kernel_text: kernel_text.into(),
            profile_text: profile_text.into(),
            profile: ModelProfile::default(),
        }
    }

    pub fn with_profile(mut self, profile: ModelProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Assemble messages in the strictly defined layer order (§11.2):
    /// Layer 1: Kernel (system message prefix, static, cached)
    /// Layer 3: Profile instructions
    /// Layer 4: Role doc
    /// Layer 5: [A4L STATE] block
    /// Layer 6: Handoff packet (L1)
    /// Layer 7: Tail verbatim turns (sliding window)
    /// Layer 8: Current user turn
    #[allow(clippy::too_many_arguments)]
    pub fn assemble(
        &self,
        role_doc: &str,
        task_id: Option<&str>,
        current_role: &str,
        skills_index: &[String],
        handoff: Option<&HandoffPacket>,
        tail_history: &[ChatMessage],
        current_turn: Option<ChatMessage>,
    ) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        // 1. Build the system prompt
        // Layer 1 + Layer 3 (Static Prefix)
        let mut system_prompt = String::with_capacity(4096);
        system_prompt.push_str(&self.kernel_text);
        system_prompt.push_str("\n\n");
        if !self.profile_text.is_empty() {
            system_prompt.push_str(&self.profile_text);
            system_prompt.push_str("\n\n");
        }

        // Layer 4: Role doc
        if !role_doc.is_empty() {
            system_prompt.push_str("--- ACTIVE ROLE INSTRUCTIONS ---\n");
            system_prompt.push_str(role_doc);
            system_prompt.push_str("\n\n");
        }

        // Layer 5: [A4L STATE] Block
        system_prompt.push_str("[A4L STATE]\n");
        system_prompt.push_str(&format!("Role: {}\n", current_role));
        if let Some(tid) = task_id {
            system_prompt.push_str(&format!("Active Task: {}\n", tid));
        } else {
            system_prompt.push_str("Active Task: none\n");
        }
        if !skills_index.is_empty() {
            system_prompt.push_str(&format!("Available Skills: {}\n", skills_index.join(", ")));
        }

        // Layer 6: Handoff Packet (L1)
        if let Some(packet) = handoff {
            system_prompt.push_str("\n[HANDOFF PACKET]\n");
            if let Ok(json_str) = serde_json::to_string(packet) {
                system_prompt.push_str(&json_str);
                system_prompt.push('\n');
            }
        }

        messages.push(ChatMessage {
            role: "system".to_string(),
            content: Some(system_prompt),
            tool_calls: None,
            tool_call_id: None,
        });

        // Layer 7: Tail turns (bounded by profile.tail_turns)
        let tail_start = tail_history.len().saturating_sub(self.profile.tail_turns);
        for turn in &tail_history[tail_start..] {
            messages.push(turn.clone());
        }

        // Layer 8: Current user turn
        if let Some(turn) = current_turn {
            messages.push(turn);
        }

        messages
    }
}
