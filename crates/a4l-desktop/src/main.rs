#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::{
    advance_task, approve_hitl, get_diff, get_gate_proofs, get_tasks, list_skills, search_memory,
    send_back_hitl,
};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_tasks,
            advance_task,
            approve_hitl,
            send_back_hitl,
            get_diff,
            get_gate_proofs,
            list_skills,
            search_memory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running A4L desktop application");
}
