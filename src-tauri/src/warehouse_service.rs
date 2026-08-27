use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::application::ObservatoryApplication;

pub const WAREHOUSE_UPDATE_EVENT: &str = "warehouse-update";
const IDLE_TICK: Duration = Duration::from_millis(500);

pub fn spawn(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    thread::Builder::new()
        .name("republic-observatory-warehouse".to_owned())
        .spawn(move || run(app_handle, application))
        .expect("warehouse projector thread must start");
}

fn run(app_handle: AppHandle, application: Arc<ObservatoryApplication>) {
    loop {
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
                thread::sleep(IDLE_TICK);
            }
        }
    }
}
