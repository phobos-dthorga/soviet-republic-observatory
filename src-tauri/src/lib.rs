mod application;
mod commands;
mod error;
mod game_vocabulary;
mod model;
mod save_archive;
mod stats_parser;
mod storage;

use tauri::Manager;

use application::ObservatoryApplication;
use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_directory = app.path().app_local_data_dir()?;
            let application = ObservatoryApplication::initialise(
                data_directory.join("republic-observatory.sqlite3"),
            )
            .map_err(|error| std::io::Error::other(error.to_string()))?;
            app.manage(AppState { application });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_setup_state,
            commands::get_latest_receiver_dataset,
            commands::get_archive_overview,
            commands::configure_directory,
            commands::observe_latest_save,
            commands::select_timeline_branch,
        ])
        .run(tauri::generate_context!())
        .expect("Republic Observatory desktop host failed");
}
