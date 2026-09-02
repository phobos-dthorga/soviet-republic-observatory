mod analysis_pack;
mod application;
mod automatic_observer;
mod broadcast;
mod bulk_work;
mod catalogue_service;
mod commands;
mod compatibility_profile;
mod compatibility_runtime;
mod compatibility_service;
mod database_reset;
mod definition_catalogue;
mod diagnostics;
mod environment;
mod error;
mod fixed_binary;
mod game_vocabulary;
mod language_pack;
mod market;
mod metric_catalogue;
mod model;
mod planning_overlay;
mod recorder_service;
mod republic_brief;
mod republic_plan;
mod research_setup;
mod research_source_download;
mod resource_catalogue;
mod save_archive;
mod setup_discovery;
mod stats_parser;
mod storage;
mod tesmio_probe;
mod theme;
mod ui_review;
mod warehouse;
mod warehouse_governor;
mod warehouse_service;

use std::sync::Arc;

use tauri::Manager;

use application::ObservatoryApplication;
use commands::AppState;
use research_setup::ResearchSetupService;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ui_review = ui_review::UiReviewStartup::parse(std::env::args_os())
        .unwrap_or_else(|error| panic!("Republic Observatory UI review startup failed: {error}"));
    let review_enabled = ui_review.context.enabled;
    let mut builder = tauri::Builder::default();
    if !review_enabled {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let data_directory = ui_review
                .data_directory
                .clone()
                .unwrap_or(app.path().app_local_data_dir()?);
            std::fs::create_dir_all(&data_directory)?;
            database_reset::apply_pending_database_reset(&data_directory)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            diagnostics::initialize(Some(&data_directory));
            diagnostics::record(
                "info",
                "application.started",
                "application_startup",
                "Republic Observatory started.",
            );
            let application = Arc::new(
                ObservatoryApplication::initialise(
                    data_directory.join("republic-observatory.sqlite3"),
                    data_directory.join("republic-observatory.duckdb"),
                )
                .map_err(|error| std::io::Error::other(error.to_string()))?,
            );
            app.manage(AppState {
                application: Arc::clone(&application),
                research_setup: Arc::new(ResearchSetupService::discover(&data_directory)),
                ui_review: ui_review.context.clone(),
                data_directory,
            });
            if !ui_review.context.enabled {
                recorder_service::spawn(app.handle().clone(), Arc::clone(&application));
                compatibility_service::spawn(app.handle().clone(), Arc::clone(&application));
                catalogue_service::spawn(app.handle().clone(), Arc::clone(&application));
                warehouse_service::spawn(app.handle().clone(), application);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_ui_review_context,
            commands::attention_cue_status,
            commands::dismiss_attention_cue,
            commands::replay_attention_cue,
            commands::replay_all_attention_cues,
            commands::get_application_settings,
            commands::update_application_preferences,
            commands::reset_application_preferences,
            commands::erase_application_databases,
            commands::get_research_setup,
            commands::get_research_report_status,
            commands::set_research_notice_accepted,
            commands::configure_research_tesmio_checkout,
            commands::download_reviewed_tesmio_source,
            commands::get_research_source_download_progress,
            commands::get_research_build_progress,
            commands::build_research_probe,
            commands::get_research_session_progress,
            commands::prepare_observation_only_session,
            commands::launch_observation_only_session,
            commands::get_setup_state,
            commands::get_latest_receiver_dataset,
            commands::get_archive_overview,
            commands::get_population_dataset,
            commands::get_republic_brief,
            commands::get_broadcast_workspace,
            commands::get_broadcast_outcome,
            commands::get_environment_workspace,
            commands::get_environment_history,
            commands::get_environment_snapshot,
            commands::save_carbon_factor_set,
            commands::select_carbon_factor_set,
            commands::rollback_carbon_factor_set,
            commands::remove_carbon_factor_set,
            commands::export_carbon_factor_set,
            commands::preview_carbon_factor_import,
            commands::apply_carbon_factor_import,
            commands::enable_environment_recording,
            commands::disable_environment_recording,
            commands::capture_environment_snapshot,
            commands::delete_live_environmental_recordings,
            commands::get_published_metric_contexts,
            commands::get_republic_plan_workspace,
            commands::get_market_workspace,
            commands::get_market_price_series,
            commands::save_market_basket,
            commands::save_market_scenario,
            commands::select_market_definition,
            commands::rollback_market_definition,
            commands::clear_market_selection,
            commands::remove_market_definition,
            commands::save_republic_plan,
            commands::activate_republic_plan,
            commands::rollback_republic_plan,
            commands::remove_republic_plan,
            commands::get_recorder_health,
            commands::choose_and_configure_directory,
            commands::observe_latest_save,
            commands::set_automatic_observation,
            commands::select_timeline_branch,
            commands::inspect_archive_observation,
            commands::return_to_branch_tip,
            commands::create_timeline_continuation,
            commands::set_timeline_branch_label,
            commands::compare_archive_observations,
            commands::get_compatibility_status,
            commands::create_local_compatibility_override,
            commands::reload_local_compatibility_override,
            commands::get_reinterpretation_progress,
            commands::reinterpret_latest_save,
            commands::get_market_indexing_progress,
            commands::recover_market_indexing,
            commands::index_available_saves_for_markets,
            commands::refresh_changed_market_data,
            commands::get_broadcast_indexing_progress,
            commands::index_available_saves_for_broadcast,
            commands::resume_broadcast_indexing,
            commands::get_environment_indexing_progress,
            commands::index_available_saves_for_environment,
            commands::resume_environment_indexing,
            commands::get_catalogue_status,
            commands::refresh_definitions,
            commands::diagnostic_log,
            commands::clear_diagnostic_log,
            commands::rebuild_warehouse,
            commands::search_catalogue,
            commands::get_resource_catalogue,
            commands::get_resource_details,
            commands::get_resource_registry_status,
            commands::list_resource_registry_snapshots,
            commands::enable_resource_registry_ingestion,
            commands::disable_resource_registry_ingestion,
            commands::get_definition_dossier,
            commands::get_production_route,
            commands::get_production_pathway,
            commands::get_production_route_coverage,
            commands::inspect_planning_overlay,
            commands::import_planning_overlay,
            commands::export_planning_overlay,
            commands::list_planning_overlays,
            commands::activate_planning_overlay,
            commands::rollback_planning_overlay,
            commands::deactivate_planning_overlay,
            commands::remove_planning_overlay,
            commands::get_warehouse_snapshot,
            commands::inspect_analysis_pack,
            commands::import_analysis_pack,
            commands::export_analysis_pack,
            commands::list_analysis_packs,
            commands::enable_analysis_pack,
            commands::disable_analysis_pack,
            commands::rollback_analysis_pack,
            commands::remove_analysis_pack,
            commands::get_analysis_pack_contributions,
            commands::language_status,
            commands::inspect_language_pack,
            commands::install_language_pack,
            commands::select_language_pack,
            commands::remove_language_pack,
            commands::export_language_pack,
            commands::handover_legacy_language_packs,
            commands::theme_status,
            commands::inspect_theme,
            commands::import_theme,
            commands::select_theme,
            commands::export_theme,
            commands::remove_theme,
        ])
        .run(tauri::generate_context!())
        .expect("Republic Observatory desktop host failed");
}
