mod commands;
mod error;
mod game_vocabulary;
mod model;
mod repository;
mod save_archive;
mod stats_parser;

use tauri::Manager;

use commands::AppState;
use repository::ObservationRepository;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_directory = app.path().app_local_data_dir()?;
            let repository = ObservationRepository::initialise(
                data_directory.join("republic-observatory.sqlite3"),
            )
            .map_err(|error| std::io::Error::other(error.to_string()))?;
            app.manage(AppState { repository });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_setup_state,
            commands::get_latest_receiver_dataset,
            commands::configure_directory,
            commands::observe_latest_save,
        ])
        .run(tauri::generate_context!())
        .expect("Republic Observatory desktop host failed");
}
