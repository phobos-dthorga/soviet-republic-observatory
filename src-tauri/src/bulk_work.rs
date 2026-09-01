use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::ObservatoryError;
use crate::model::BackgroundWorkPriority;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ContentionWait {
    pub waited_ms: u64,
    pub retries: u32,
}

#[derive(Debug, Default)]
pub(crate) struct BulkWorkCoordinator {
    critical_writer: Mutex<()>,
    recorder_waiters: AtomicUsize,
}

impl BulkWorkCoordinator {
    pub fn recorder_write(&self) -> Result<MutexGuard<'_, ()>, ObservatoryError> {
        let waiting = RecorderWaiting::new(&self.recorder_waiters);
        let guard = self
            .critical_writer
            .lock()
            .map_err(|_| ObservatoryError::StorageUnavailable)?;
        drop(waiting);
        Ok(guard)
    }

    pub fn background_write(
        &self,
        patience: Duration,
        priority: BackgroundWorkPriority,
    ) -> Result<(MutexGuard<'_, ()>, ContentionWait), ObservatoryError> {
        let started = Instant::now();
        let mut retries = 0_u32;
        loop {
            if self.recorder_waiters.load(Ordering::Acquire) == 0 {
                match self.critical_writer.try_lock() {
                    Ok(guard) => {
                        return Ok((
                            guard,
                            ContentionWait {
                                waited_ms: elapsed_ms(started),
                                retries,
                            },
                        ));
                    }
                    Err(TryLockError::Poisoned(_)) => {
                        return Err(ObservatoryError::StorageUnavailable);
                    }
                    Err(TryLockError::WouldBlock) => {}
                }
            }
            if started.elapsed() >= patience {
                return Err(ObservatoryError::StorageBusy);
            }
            retries = retries.saturating_add(1);
            thread::sleep(wait_interval(priority).min(patience.saturating_sub(started.elapsed())));
        }
    }

    pub fn cooperative_yield(&self, priority: BackgroundWorkPriority) {
        let delay = match priority {
            BackgroundWorkPriority::Gentle => Duration::from_millis(80),
            BackgroundWorkPriority::Balanced => Duration::from_millis(20),
            BackgroundWorkPriority::FinishSooner => Duration::ZERO,
        };
        if !delay.is_zero() || self.recorder_waiters.load(Ordering::Acquire) > 0 {
            thread::sleep(delay.max(Duration::from_millis(1)));
        }
    }
}

fn wait_interval(priority: BackgroundWorkPriority) -> Duration {
    match priority {
        BackgroundWorkPriority::Gentle => Duration::from_millis(40),
        BackgroundWorkPriority::Balanced => Duration::from_millis(20),
        BackgroundWorkPriority::FinishSooner => Duration::from_millis(10),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

struct RecorderWaiting<'a> {
    waiters: &'a AtomicUsize,
}

impl<'a> RecorderWaiting<'a> {
    fn new(waiters: &'a AtomicUsize) -> Self {
        waiters.fetch_add(1, Ordering::AcqRel);
        Self { waiters }
    }
}

impl Drop for RecorderWaiting<'_> {
    fn drop(&mut self) {
        self.waiters.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::BulkWorkCoordinator;
    use crate::model::BackgroundWorkPriority;

    #[test]
    fn background_work_times_out_instead_of_waiting_forever() {
        let coordinator = Arc::new(BulkWorkCoordinator::default());
        let recorder = coordinator.recorder_write().expect("recorder lease");
        let background = Arc::clone(&coordinator);
        let result = thread::spawn(move || {
            background
                .background_write(
                    Duration::from_millis(25),
                    BackgroundWorkPriority::FinishSooner,
                )
                .map(|_| "acquired")
                .map_err(|error| error.code())
        })
        .join()
        .expect("background thread");
        assert_eq!(result.expect_err("must pause"), "storage_busy");
        drop(recorder);
    }
}
