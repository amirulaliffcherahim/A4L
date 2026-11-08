use serde_json::{json, Value};

pub fn tasks_json_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "A4L Tasks Schema",
        "type": "object",
        "required": ["version", "queue"],
        "properties": {
            "version": { "type": "integer" },
            "queue": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["id", "title", "created_at", "updated_at", "state", "todos"],
                    "properties": {
                        "id": { "type": "string" },
                        "title": { "type": "string" },
                        "created_at": { "type": "string", "format": "date-time" },
                        "updated_at": { "type": "string", "format": "date-time" },
                        "state": {
                            "type": "object",
                            "required": ["status", "role", "attempt"],
                            "properties": {
                                "status": {
                                    "type": "string",
                                    "enum": ["planned", "in_progress", "review", "verifying", "awaiting_hitl", "completed", "failed", "sent_back"]
                                },
                                "role": {
                                    "type": "string",
                                    "enum": ["planner", "maker", "checker", "human"]
                                },
                                "attempt": { "type": "integer" },
                                "context_bundle": { "type": ["string", "null"] }
                            }
                        },
                        "todos": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": ["id", "text", "status"],
                                "properties": {
                                    "id": { "type": "string" },
                                    "text": { "type": "string" },
                                    "status": {
                                        "type": "string",
                                        "enum": ["pending", "done", "verified"]
                                    },
                                    "verified_at": { "type": ["string", "null"] }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

pub fn failure_bundle_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "A4L Failure Bundle Schema",
        "type": "object",
        "required": ["task_id", "attempt", "trip_reason", "error_hashes", "quarantine_branch"],
        "properties": {
            "task_id": { "type": "string" },
            "attempt": { "type": "integer" },
            "trip_reason": {
                "type": "string",
                "enum": ["duplicate_error", "diff_oscillation", "max_attempts"]
            },
            "error_hashes": { "type": "array", "items": { "type": "string" } },
            "last_gate_logs": { "type": "array", "items": { "type": "string" } },
            "quarantine_branch": { "type": "string" },
            "created_at": { "type": "string", "format": "date-time" }
        }
    })
}

pub fn checker_verdict_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "A4L Checker Verdict Schema",
        "type": "object",
        "required": ["task_id", "attempt", "verdict", "findings", "evidence"],
        "properties": {
            "task_id": { "type": "string" },
            "attempt": { "type": "integer" },
            "verdict": { "type": "string", "enum": ["pass", "fail"] },
            "findings": { "type": "array", "items": { "type": "string" } },
            "evidence": {
                "type": "object",
                "required": ["diff_ref", "gate_logs"],
                "properties": {
                    "diff_ref": { "type": "string" },
                    "gate_logs": { "type": "array", "items": { "type": "string" } }
                }
            }
        }
    })
}

pub fn handoff_packet_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "A4L Handoff Packet Schema",
        "type": "object",
        "required": ["v", "task_id", "seq", "trigger", "from", "facts", "note"],
        "properties": {
            "v": { "type": "integer" },
            "task_id": { "type": "string" },
            "seq": { "type": "integer" },
            "trigger": { "type": "string" },
            "from": {
                "type": "object",
                "required": ["role", "mode", "model"],
                "properties": {
                    "role": { "type": "string" },
                    "mode": { "type": "string" },
                    "model": { "type": "string" }
                }
            },
            "facts": { "type": "object" },
            "note": { "type": "object" },
            "note_stale": { "type": "boolean" },
            "refs": { "type": "object" }
        }
    })
}
