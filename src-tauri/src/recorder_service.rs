use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::thread;
use std::time::Duration;

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::application::ObservatoryApplication;
use crate::model::RecorderDiscoverySource;
use crate::storage::now_ms;

pub const RECORDER_UPDATE_EVENT: &str = "recorder-update";

const SERVICE_TICK: Duration = Duration::from_millis(500);
const RECONCILIATION_INTERVAL_MS: i64 = 15_000;

pub fn spawn(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    thread::Builder::new()
        .name("republic-observatory-recorder".to_owned())
        .spawn(move || run(app_handle, application))
        .expect("native recorder thread must start");
}

fn run(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    let (event_sender, event_receiver) = channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = event_sender.send(event);
    })
    .ok();
    let mut watched_directory: Option<PathBuf> = None;
    let mut last_reconciliation_ms = 0;
    let mut last_resource_registry_sync_ms = 0;
    let mut last_enabled = false;

    loop {
        let Ok((enabled, configured_directory)) = application.recorder_configuration() else {
            thread::sleep(SERVICE_TICK);
            continue;
        };
        let usable_directory = configured_directory.filter(|path| path.is_dir());
        if update_watch(
            watcher.as_mut(),
            &mut watched_directory,
            enabled.then_some(usable_directory.as_deref()).flatten(),
        ) {
            last_reconciliation_ms = 0;
        }

        let filesystem_event = receive_relevant_event(&event_receiver);
        let now = now_ms();
        if now - last_resource_registry_sync_ms >= RECONCILIATION_INTERVAL_MS {
            let _ = application.sync_resource_registry();
            let _ = application.sync_environment_validation();
            last_resource_registry_sync_ms = now;
        }
        if filesystem_event {
            let _ = application.note_recorder_filesystem_event(now);
        }

        let processing_candidate = enabled
            && application.recorder_health().ok().is_some_and(|health| {
                matches!(
                    health.observer.phase,
                    crate::model::AutomaticObserverPhase::WaitingForStability
                        | crate::model::AutomaticObserverPhase::Retrying
                )
            });
        let reconciliation_due = now - last_reconciliation_ms >= RECONCILIATION_INTERVAL_MS;
        let configuration_changed = enabled != last_enabled;

        if enabled
            && usable_directory.is_some()
            && (filesystem_event
                || processing_candidate
                || reconciliation_due
                || configuration_changed)
        {
            let source = if filesystem_event {
                RecorderDiscoverySource::FilesystemEvent
            } else {
                RecorderDiscoverySource::Reconciliation
            };
            if let Ok(update) = application.poll_automatic_observation_from(source)
                && let Ok(event) = application.recorder_update(update.import_result)
            {
                let _ = app_handle.emit(RECORDER_UPDATE_EVENT, event);
            }
            if reconciliation_due {
                last_reconciliation_ms = now;
            }
        } else if configuration_changed && let Ok(event) = application.recorder_update(None) {
            let _ = app_handle.emit(RECORDER_UPDATE_EVENT, event);
        }
        last_enabled = enabled;
    }
}

fn update_watch(
    watcher: Option<&mut RecommendedWatcher>,
    watched_directory: &mut Option<PathBuf>,
    requested_directory: Option<&Path>,
) -> bool {
    if watched_directory.as_deref() == requested_directory {
        return false;
    }
    let Some(watcher) = watcher else {
        *watched_directory = requested_directory.map(Path::to_path_buf);
        return true;
    };
    if let Some(current) = watched_directory.take() {
        let _ = watcher.unwatch(&current);
    }
    if let Some(requested) = requested_directory
        && watcher
            .watch(requested, RecursiveMode::NonRecursive)
            .is_ok()
    {
        *watched_directory = Some(requested.to_path_buf());
    }
    true
}

fn receive_relevant_event(receiver: &Receiver<notify::Result<Event>>) -> bool {
    let mut relevant = match receiver.recv_timeout(SERVICE_TICK) {
        Ok(Ok(event)) => event_is_relevant(&event),
        Ok(Err(_)) | Err(RecvTimeoutError::Timeout) => false,
        Err(RecvTimeoutError::Disconnected) => false,
    };
    while let Ok(event) = receiver.try_recv() {
        if event.as_ref().is_ok_and(event_is_relevant) {
            relevant = true;
        }
    }
    relevant
}

fn event_is_relevant(event: &Event) -> bool {
    event.paths.is_empty()
        || event.paths.iter().any(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        })
}

#[cfg(test)]
mod tests {
    use notify::{Event, EventKind};

    use super::event_is_relevant;

    #[test]
    fn only_zip_related_events_trigger_immediate_reconciliation() {
        let zip = Event::new(EventKind::Any).add_path("save.zip".into());
        let text = Event::new(EventKind::Any).add_path("notes.txt".into());
        assert!(event_is_relevant(&zip));
        assert!(!event_is_relevant(&text));
    }
}
