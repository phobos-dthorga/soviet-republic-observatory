use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::ObservatoryError;
use crate::model::{WarehouseWriteActivity, WarehouseWriteKind, WarehouseWriteStage};

const MAX_CATALOGUE_ROWS: u64 = 6_000_000;
const MAX_OBSERVATION_ROWS: u64 = 5_000_000;
const MAX_MARKET_ROWS: u64 = 5_000_000;
const MAX_BROADCAST_ROWS: u64 = 1_000_000;
const MAX_OVERLAY_ROWS: u64 = 4_608;
const MAX_BRANCH_MEMBERSHIP_ROWS: u64 = 1_000_000;
const MAX_BACKOFF: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WarehouseGovernorSnapshot {
    pub active_write: Option<WarehouseWriteActivity>,
    pub consecutive_failures: u32,
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Default)]
struct WarehouseGovernorState {
    active_write: Option<WarehouseWriteActivity>,
    consecutive_failures: u32,
    retry_not_before_ms: Option<i64>,
}

#[derive(Debug, Default)]
pub(crate) struct WarehouseGovernor {
    state: Mutex<WarehouseGovernorState>,
}

impl WarehouseGovernor {
    pub fn begin(
        &self,
        kind: WarehouseWriteKind,
        rows_total: u64,
    ) -> Result<WarehouseWritePermit<'_>, ObservatoryError> {
        if rows_total > row_limit(kind) {
            return Err(ObservatoryError::WarehouseWriteLimit);
        }
        let timestamp = now_ms();
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.active_write.is_some() {
            return Err(ObservatoryError::WarehouseUnavailable);
        }
        state.active_write = Some(WarehouseWriteActivity {
            kind,
            stage: WarehouseWriteStage::Staging,
            started_at_ms: timestamp,
            updated_at_ms: timestamp,
            rows_processed: 0,
            rows_total,
        });
        Ok(WarehouseWritePermit {
            governor: self,
            completed: false,
        })
    }

    pub fn note_failure(&self) {
        let timestamp = now_ms();
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.consecutive_failures = state.consecutive_failures.saturating_add(1);
        let delay = failure_backoff(state.consecutive_failures);
        state.retry_not_before_ms = Some(timestamp.saturating_add(delay.as_millis() as i64));
    }

    pub fn note_success(&self) {
        self.complete();
    }

    pub fn retry_delay(&self) -> Duration {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state
            .retry_not_before_ms
            .map(|retry_at| retry_at.saturating_sub(now_ms()).max(0) as u64)
            .map(Duration::from_millis)
            .unwrap_or_default()
    }

    pub fn snapshot(&self) -> WarehouseGovernorSnapshot {
        let state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let retry_after_ms = state
            .retry_not_before_ms
            .map(|retry_at| retry_at.saturating_sub(now_ms()).max(0) as u64)
            .filter(|delay| *delay > 0);
        WarehouseGovernorSnapshot {
            active_write: state.active_write.clone(),
            consecutive_failures: state.consecutive_failures,
            retry_after_ms,
        }
    }

    fn progress(&self, stage: WarehouseWriteStage, rows_processed: u64) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(active) = &mut state.active_write {
            active.stage = stage;
            active.rows_processed = rows_processed.min(active.rows_total);
            active.updated_at_ms = now_ms();
        }
    }

    fn complete(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.active_write = None;
        state.consecutive_failures = 0;
        state.retry_not_before_ms = None;
    }

    fn abandon(&self) {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .active_write = None;
    }
}

pub(crate) struct WarehouseWritePermit<'a> {
    governor: &'a WarehouseGovernor,
    completed: bool,
}

impl WarehouseWritePermit<'_> {
    pub fn progress(&self, stage: WarehouseWriteStage, rows_processed: u64) {
        self.governor.progress(stage, rows_processed);
    }

    pub fn complete(mut self) {
        self.governor.complete();
        self.completed = true;
    }
}

impl Drop for WarehouseWritePermit<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.governor.abandon();
        }
    }
}

fn row_limit(kind: WarehouseWriteKind) -> u64 {
    match kind {
        WarehouseWriteKind::CataloguePublication => MAX_CATALOGUE_ROWS,
        WarehouseWriteKind::ObservationProjection => MAX_OBSERVATION_ROWS,
        WarehouseWriteKind::MarketProjection => MAX_MARKET_ROWS,
        WarehouseWriteKind::BroadcastProjection => MAX_BROADCAST_ROWS,
        WarehouseWriteKind::OverlayProjection => MAX_OVERLAY_ROWS,
        WarehouseWriteKind::BranchMembershipProjection => MAX_BRANCH_MEMBERSHIP_ROWS,
        WarehouseWriteKind::ObservationRebuild => 0,
    }
}

fn failure_backoff(consecutive_failures: u32) -> Duration {
    let exponent = consecutive_failures.saturating_sub(1).min(6);
    Duration::from_millis(500_u64.saturating_mul(1_u64 << exponent)).min(MAX_BACKOFF)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_writes_before_they_reach_duckdb() {
        let governor = WarehouseGovernor::default();
        assert!(matches!(
            governor.begin(WarehouseWriteKind::OverlayProjection, MAX_OVERLAY_ROWS + 1),
            Err(ObservatoryError::WarehouseWriteLimit)
        ));
        assert!(governor.snapshot().active_write.is_none());
    }

    #[test]
    fn reports_progress_and_clears_successful_activity() {
        let governor = WarehouseGovernor::default();
        let permit = governor
            .begin(WarehouseWriteKind::ObservationProjection, 8_000)
            .expect("write permit");
        permit.progress(WarehouseWriteStage::Merging, 8_000);
        let active = governor.snapshot().active_write.expect("active write");
        assert_eq!(active.rows_processed, 8_000);
        assert_eq!(active.stage, WarehouseWriteStage::Merging);
        permit.complete();
        assert_eq!(governor.snapshot(), WarehouseGovernorSnapshot::default());
    }

    #[test]
    fn failures_apply_exponential_bounded_backoff() {
        assert_eq!(failure_backoff(1), Duration::from_millis(500));
        assert_eq!(failure_backoff(2), Duration::from_secs(1));
        assert_eq!(failure_backoff(3), Duration::from_secs(2));
        assert_eq!(failure_backoff(20), MAX_BACKOFF);
        let governor = WarehouseGovernor::default();
        governor.note_failure();
        let snapshot = governor.snapshot();
        assert_eq!(snapshot.consecutive_failures, 1);
        assert!(snapshot.retry_after_ms.is_some_and(|delay| delay <= 500));
        governor.note_success();
        assert_eq!(governor.snapshot(), WarehouseGovernorSnapshot::default());
    }

    #[test]
    fn late_failure_accounting_does_not_clear_a_new_active_writer() {
        let governor = WarehouseGovernor::default();
        let permit = governor
            .begin(WarehouseWriteKind::CataloguePublication, 10)
            .expect("new active writer");

        governor.note_failure();

        let snapshot = governor.snapshot();
        assert!(snapshot.active_write.is_some());
        assert_eq!(snapshot.consecutive_failures, 1);
        permit.complete();
        assert_eq!(governor.snapshot(), WarehouseGovernorSnapshot::default());
    }

    #[test]
    fn expired_backoff_clamps_to_zero() {
        let governor = WarehouseGovernor::default();
        governor
            .state
            .lock()
            .expect("governor state")
            .retry_not_before_ms = Some(now_ms().saturating_sub(1));

        assert!(governor.retry_delay().is_zero());
        assert!(governor.snapshot().retry_after_ms.is_none());
    }
}
