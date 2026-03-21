use parking_lot::RwLock;
use std::collections::HashMap;
use tokio::sync::mpsc;

use solverforge::{HardSoftDecimalScore, SolverManager, SolverStatus};

use crate::domain::EmployeeSchedule;

// Static manager — must be 'static for SolverManager::solve.
static MANAGER: SolverManager<EmployeeSchedule> = SolverManager::new();

struct JobState {
    slot_id: usize,
    latest: Option<EmployeeSchedule>,
    score: Option<HardSoftDecimalScore>,
    receiver: mpsc::UnboundedReceiver<(EmployeeSchedule, HardSoftDecimalScore)>,
    status: SolverStatus,
}

/// Manages solving jobs using the framework SolverManager.
pub struct SolverService {
    jobs: RwLock<HashMap<String, JobState>>,
}

impl SolverService {
    pub fn new() -> Self {
        Self { jobs: RwLock::new(HashMap::new()) }
    }

    pub fn start_solving(&self, id: String, schedule: EmployeeSchedule) {
        let (slot_id, receiver) = MANAGER.solve(schedule);
        let state = JobState {
            slot_id,
            latest: None,
            score: None,
            receiver,
            status: SolverStatus::Solving,
        };
        self.jobs.write().insert(id, state);
    }

    // Polls the channel and calls `f` with the latest schedule.
    pub fn with_snapshot<R>(
        &self,
        id: &str,
        f: impl FnOnce(&EmployeeSchedule, Option<HardSoftDecimalScore>, SolverStatus) -> R,
    ) -> Option<R> {
        let mut jobs = self.jobs.write();
        let state = jobs.get_mut(id)?;
        while let Ok((solution, score)) = state.receiver.try_recv() {
            state.latest = Some(solution);
            state.score = Some(score);
        }
        state.status = MANAGER.get_status(state.slot_id);
        Some(f(state.latest.as_ref()?, state.score, state.status))
    }

    pub fn has_job(&self, id: &str) -> bool {
        self.jobs.read().contains_key(id)
    }

    pub fn list_jobs(&self) -> Vec<String> {
        self.jobs.read().keys().cloned().collect()
    }

    pub fn stop_solving(&self, id: &str) -> bool {
        let jobs = self.jobs.read();
        if let Some(state) = jobs.get(id) {
            return MANAGER.terminate_early(state.slot_id);
        }
        false
    }

    pub fn remove_job(&self, id: &str) -> bool {
        if let Some(state) = self.jobs.write().remove(id) {
            MANAGER.free_slot(state.slot_id);
            return true;
        }
        false
    }
}

impl Default for SolverService {
    fn default() -> Self {
        Self::new()
    }
}
