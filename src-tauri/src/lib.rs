mod application;
mod automatic_observer;
mod commands;
mod error;
mod game_vocabulary;
mod model;
mod recorder_service;
mod save_archive;
mod stats_parser;
mod storage;

use std::sync::Arc;

use tauri::Manager;

use application::ObservatoryApplication;
use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_directory = app.path().app_local_data_dir()?;
            let application = Arc::new(
                ObservatoryApplication::initialise(
                    data_directory.join("republic-observatory.sqlite3"),
                )
                .map_err(|error| std::io::Error::other(error.to_string()))?,
            );
            app.manage(AppState {
                application: Arc::clone(&application),
            });
            recorder_service::spawn(app.handle().clone(), application);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_setup_state,
            commands::get_latest_receiver_dataset,
            commands::get_archive_overview,
            commands::get_recorder_health,
            commands::configure_directory,
            commands::observe_latest_save,
            commands::set_automatic_observation,
            commands::select_timeline_branch,
            commands::compare_archive_observations,
        ])
        .run(tauri::generate_context!())
        .expect("Republic Observatory desktop host failed");
}
