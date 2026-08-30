use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::thread;
use std::time::Duration;

use notify::{Event, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::application::ObservatoryApplication;
use crate::model::CatalogueRefreshTrigger;
use crate::storage::now_ms;

pub const COMPATIBILITY_UPDATE_EVENT: &str = "compatibility-update";
pub const REINTERPRETATION_PROGRESS_EVENT: &str = "compatibility-reinterpretation-progress";
const SERVICE_TICK: Duration = Duration::from_millis(250);
const DEBOUNCE_MS: i64 = 750;

pub fn spawn(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    thread::Builder::new()
        .name("republic-observatory-compatibility".to_owned())
        .spawn(move || run(app_handle, application))
        .expect("compatibility observer thread must start");
}

pub fn schedule_catalogue_refresh(
    app_handle: AppHandle,
    application: Arc<ObservatoryApplication>,
    update: &crate::model::CompatibilityUpdate,
) {
    if !update.definition_mapping_changed
        || application
            .catalogue_configuration()
            .ok()
            .flatten()
            .is_none()
    {
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        refresh_catalogue(&app_handle, &application);
    });
}

fn refresh_catalogue(app_handle: &AppHandle, application: &ObservatoryApplication) {
    if application
        .catalogue_configuration()
        .ok()
        .flatten()
        .is_none()
    {
        return;
    }
    let _ = application.refresh_catalogue(CatalogueRefreshTrigger::Filesystem, |progress| {
        let _ = app_handle.emit(crate::catalogue_service::CATALOGUE_PROGRESS_EVENT, progress);
    });
    if let Ok(status) = application.catalogue_status() {
        let _ = app_handle.emit(crate::catalogue_service::CATALOGUE_UPDATE_EVENT, status);
    }
    if let Ok(status) = application.compatibility_status() {
        let _ = app_handle.emit(
            COMPATIBILITY_UPDATE_EVENT,
            crate::model::CompatibilityUpdate {
                status,
                profile_changed: false,
                definition_mapping_changed: false,
            },
        );
    }
}

fn run(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    let (sender, receiver) = channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .ok();
    if let (Some(watcher), Some(root)) = (watcher.as_mut(), application.compatibility_watch_root())
    {
        let _ = watcher.watch(&root, RecursiveMode::NonRecursive);
    }
    let mut reload_due_at = None;
    loop {
        match receiver.recv_timeout(SERVICE_TICK) {
            Ok(Ok(event)) if event_is_relevant(&event) => {
                reload_due_at = Some(now_ms().saturating_add(DEBOUNCE_MS));
            }
            Ok(_) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(event) = receiver.try_recv() {
            if event.as_ref().is_ok_and(event_is_relevant) {
                reload_due_at = Some(now_ms().saturating_add(DEBOUNCE_MS));
            }
        }
        if reload_due_at.is_some_and(|due| now_ms() >= due) {
            reload_due_at = None;
            if let Ok(update) = application.reload_compatibility() {
                let _ = app_handle.emit(COMPATIBILITY_UPDATE_EVENT, &update);
                if update.definition_mapping_changed {
                    refresh_catalogue(&app_handle, &application);
                }
            }
        }
    }
}

fn event_is_relevant(event: &Event) -> bool {
    event.paths.is_empty()
        || event
            .paths
            .iter()
            .any(|path| is_local_profile_path(path.as_path()))
}

fn is_local_profile_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("local.rocompat.json"))
}

#[cfg(test)]
mod tests {
    use notify::{Event, EventKind};

    use super::event_is_relevant;

    #[test]
    fn watches_only_the_single_local_compatibility_file() {
        assert!(event_is_relevant(
            &Event::new(EventKind::Any).add_path("local.rocompat.json".into())
        ));
        assert!(!event_is_relevant(
            &Event::new(EventKind::Any).add_path("another.rocompat.json".into())
        ));
    }
}
