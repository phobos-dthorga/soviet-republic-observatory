use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::application::ObservatoryApplication;

pub const WAREHOUSE_UPDATE_EVENT: &str = "warehouse-update";
const IDLE_TICK: Duration = Duration::from_millis(500);
const PROGRESS_TICK: Duration = Duration::from_millis(750);

pub fn spawn(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    let progress_handle = app_handle.clone();
    let progress_application = Arc::clone(&application);
    thread::Builder::new()
        .name("republic-observatory-warehouse".to_owned())
        .spawn(move || run(app_handle, application))
        .expect("warehouse projector thread must start");
    thread::Builder::new()
        .name("republic-observatory-warehouse-progress".to_owned())
        .spawn(move || monitor_progress(progress_handle, progress_application))
        .expect("warehouse progress thread must start");
}

fn run(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    loop {
        let retry_delay = application.warehouse_retry_delay();
        if !retry_delay.is_zero() {
            thread::sleep(retry_delay.min(IDLE_TICK));
            continue;
        }
        match application.process_next_projection_job() {
            Ok(true) => {
                if let Ok(status) = application.catalogue_status() {
                    let _ = app_handle.emit(WAREHOUSE_UPDATE_EVENT, status);
                }
            }
            Ok(false) => thread::sleep(IDLE_TICK),
            Err(_) => {
                if let Ok(status) = application.catalogue_status() {
                    let _ = app_handle.emit(WAREHOUSE_UPDATE_EVENT, status);
                }
            }
        }
    }
}

fn monitor_progress(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    loop {
        thread::sleep(PROGRESS_TICK);
        if let Ok(status) = application.catalogue_status()
            && (status.warehouse.active_write.is_some()
                || status.warehouse.retry_after_ms.is_some())
        {
            let _ = app_handle.emit(WAREHOUSE_UPDATE_EVENT, status);
        }
    }
}
