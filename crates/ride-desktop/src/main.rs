// Prevents an additional console window from opening on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::DesktopState;
use std::sync::Mutex;
use tracing::info;

fn main() {
    tracing_subscriber::fmt::init();

    info!(
        "Starting {} v{}",
        ride_core::RIDE_APP_NAME,
        ride_core::RIDE_VERSION
    );

    tauri::Builder::default()
        .manage(Mutex::new(DesktopState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::get_model_status,
            commands::send_agent_message,
            commands::get_active_skills,
            commands::toggle_skill,
            commands::get_diff_preview,
            commands::stage_diff_preview,
            commands::apply_diff,
            commands::rollback_diff,
            commands::get_active_layout,
            commands::set_active_layout,
            commands::get_moe_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running R.I.D.E desktop application");
}
