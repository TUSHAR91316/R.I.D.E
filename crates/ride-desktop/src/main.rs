// Prevents an additional console window from opening on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::DesktopState;
use std::sync::Mutex;
use tauri::Emitter;
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
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let (event_tx, mut event_rx) = tokio::sync::broadcast::channel(32);
                let cancel = tokio_util::sync::CancellationToken::new();

                let detector_runner = ride_discovery::ModelDetector::default();
                let cancel_runner = cancel.clone();
                tauri::async_runtime::spawn(async move {
                    detector_runner
                        .run_discovery_loop(event_tx, cancel_runner, std::time::Duration::from_secs(2))
                        .await;
                });

                while let Ok(event) = event_rx.recv().await {
                    let _ = handle.emit("model://event", &event);
                }
            });
            Ok(())
        })
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
            commands::save_file,
            commands::read_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running R.I.D.E desktop application");
}
