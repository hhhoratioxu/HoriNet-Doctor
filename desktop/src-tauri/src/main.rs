#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod commands;
use commands::AppState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let store = horinet_core::storage::Store::new(app.path().app_local_data_dir()?)?;
            app.manage(AppState::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::inspect_network,
            commands::start_diagnosis,
            commands::start_nat,
            commands::start_monitor,
            commands::start_trace,
            commands::start_dns,
            commands::start_proxy,
            commands::test_tcp,
            commands::cancel_job,
            commands::pause_monitor,
            commands::list_history,
            commands::get_report,
            commands::delete_history,
            commands::clear_history,
            commands::export_report,
            commands::copy_summary,
            commands::export_trace,
            commands::open_repository
        ])
        .build(tauri::generate_context!())
        .expect("failed to initialize HoriNet Doctor")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<AppState>().cancel_all();
            }
        });
}
