use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread;
use std::time::Duration;

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::application::ObservatoryApplication;
use crate::model::CatalogueRefreshTrigger;
use crate::storage::now_ms;

pub const CATALOGUE_UPDATE_EVENT: &str = "catalogue-update";
pub const CATALOGUE_PROGRESS_EVENT: &str = "catalogue-progress";
const SERVICE_TICK: Duration = Duration::from_millis(500);
const DEBOUNCE_MS: i64 = 5_000;

pub fn spawn(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    thread::Builder::new()
        .name("republic-observatory-catalogue".to_owned())
        .spawn(move || run(app_handle, application))
        .expect("catalogue observer thread must start");
}

fn run(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    let (sender, receiver) = channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .ok();
    let mut watched_roots = Vec::new();
    let mut configured_directory: Option<PathBuf> = None;
    let mut refresh_due_at: Option<i64> = None;
    let mut refresh_trigger = CatalogueRefreshTrigger::Startup;

    loop {
        let current_directory = application
            .catalogue_configuration()
            .ok()
            .flatten()
            .filter(|path| path.is_dir());
        let requested_roots = application.catalogue_watch_roots().unwrap_or_default();
        if current_directory != configured_directory || requested_roots != watched_roots {
            configured_directory = current_directory;
            update_watches(watcher.as_mut(), &mut watched_roots, requested_roots);
            refresh_due_at = configured_directory.as_ref().map(|_| now_ms());
            refresh_trigger = CatalogueRefreshTrigger::Startup;
        }

        if receive_relevant_event(&receiver) {
            let event_at = now_ms();
            let _ = application.note_catalogue_filesystem_event(event_at);
            refresh_due_at = Some(event_at.saturating_add(DEBOUNCE_MS));
            refresh_trigger = CatalogueRefreshTrigger::Filesystem;
        }

        if refresh_due_at.is_some_and(|due| now_ms() >= due) {
            let _ = application.refresh_catalogue(refresh_trigger, |progress| {
                let _ = app_handle.emit(CATALOGUE_PROGRESS_EVENT, progress);
            });
            if let Ok(status) = application.catalogue_status() {
                let _ = app_handle.emit(CATALOGUE_UPDATE_EVENT, status);
            }
            refresh_due_at = None;
        }
    }
}

fn update_watches(
    watcher: Option<&mut RecommendedWatcher>,
    current: &mut Vec<PathBuf>,
    requested: Vec<PathBuf>,
) {
    if *current == requested {
        return;
    }
    let Some(watcher) = watcher else {
        *current = requested;
        return;
    };
    for root in current.drain(..) {
        let _ = watcher.unwatch(&root);
    }
    for root in requested {
        if watcher.watch(&root, RecursiveMode::Recursive).is_ok() {
            current.push(root);
        }
    }
}

fn receive_relevant_event(receiver: &Receiver<notify::Result<Event>>) -> bool {
    let mut relevant = match receiver.recv_timeout(SERVICE_TICK) {
        Ok(Ok(event)) => event_is_relevant(&event),
        Ok(Err(_)) | Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => false,
    };
    while let Ok(event) = receiver.try_recv() {
        relevant |= event.as_ref().is_ok_and(event_is_relevant);
    }
    relevant
}

fn event_is_relevant(event: &Event) -> bool {
    event.paths.is_empty()
        || event.paths.iter().any(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.eq_ignore_ascii_case("building.ini")
                        || name.eq_ignore_ascii_case("script.ini")
                        || name.eq_ignore_ascii_case("workshopconfig.ini")
                        || name.eq_ignore_ascii_case("appmanifest_784150.acf")
                })
        })
}

#[cfg(test)]
mod tests {
    use notify::{Event, EventKind};

    use super::event_is_relevant;

    #[test]
    fn definition_events_are_bounded_to_known_files() {
        assert!(event_is_relevant(
            &Event::new(EventKind::Any).add_path("building.ini".into())
        ));
        assert!(!event_is_relevant(
            &Event::new(EventKind::Any).add_path("mesh.mtl".into())
        ));
    }
}
