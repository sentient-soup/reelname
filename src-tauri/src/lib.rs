mod commands;
mod db;
mod matcher;
mod models;
mod naming;
mod parser;
mod scanner;
mod tmdb;
mod transfer;
mod tray;

use tauri::Manager;

pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            db::initialize(app.handle())?;
            tray::create_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                #[allow(unused_must_use)]
                { window.hide(); }
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_directory,
            commands::match_groups,
            commands::get_groups,
            commands::get_group,
            commands::update_group,
            commands::delete_group,
            commands::get_jobs,
            commands::get_job,
            commands::update_job,
            commands::delete_job,
            commands::bulk_action,
            commands::get_seasons,
            commands::get_season_episodes,
            commands::search_tmdb,
            commands::get_settings,
            commands::update_settings,
            commands::get_destinations,
            commands::create_destination,
            commands::update_destination,
            commands::delete_destination,
            commands::test_ssh_connection,
            commands::start_transfer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
