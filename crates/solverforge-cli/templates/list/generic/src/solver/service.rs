use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::{broadcast, mpsc};

use solverforge::{HardSoftScore, SolverManager, SolverStatus};

use crate::domain::Plan;

// Static manager — must be 'static for SolverManager::solve.
static MANAGER: SolverManager<Plan> = SolverManager::new();

fn sse_payload(score: Option<HardSoftScore>, status: SolverStatus, mps: u64) -> String {
    let score_str = score.map(|s| format!("{}", s));
    let status_str = match status {
        SolverStatus::Solving => "SOLVING",
        SolverStatus::NotSolving => "NOT_SOLVING",
    };
    match score_str {
        Some(s) => format!(r#"{{"score":"{}","solverStatus":"{}","movesPerSecond":{}}}"#, s, status_str, mps),
        None => format!(r#"{{"score":null,"solverStatus":"{}","movesPerSecond":{}}}"#, status_str, mps),
    }
}

struct JobState {
    slot_id: usize,
    latest: Option<Plan>,
    score: Option<HardSoftScore>,
    status: SolverStatus,
    sse_tx: broadcast::Sender<String>,
}

/// Manages solving jobs using the framework SolverManager.
pub struct SolverService {
    jobs: Arc<RwLock<HashMap<String, JobState>>>,
}

impl SolverService {
    pub fn new() -> Self {
        Self { jobs: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub fn start_solving(&self, id: String, plan: Plan) {
        let (slot_id, receiver) = MANAGER.solve(plan);
        let (sse_tx, _) = broadcast::channel(64);
        let state = JobState {
            slot_id,
            latest: None,
            score: None,
            status: SolverStatus::Solving,
            sse_tx: sse_tx.clone(),
        };
        self.jobs.write().insert(id.clone(), state);

        let jobs = Arc::clone(&self.jobs);
        tokio::spawn(async move {
            drain_receiver(jobs, id, slot_id, sse_tx, receiver).await;
        });
    }

    pub fn with_snapshot<R>(
        &self,
        id: &str,
        f: impl FnOnce(&Plan, Option<HardSoftScore>, SolverStatus) -> R,
    ) -> Option<R> {
        let jobs = self.jobs.read();
        let state = jobs.get(id)?;
        Some(f(state.latest.as_ref()?, state.score, state.status))
    }

    pub fn subscribe(&self, id: &str) -> Option<broadcast::Receiver<String>> {
        self.jobs.read().get(id).map(|s| s.sse_tx.subscribe())
    }

    pub fn sse_snapshot(&self, id: &str) -> Option<String> {
        let jobs = self.jobs.read();
        let state = jobs.get(id)?;
        Some(sse_payload(state.score, state.status, 0))
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

async fn drain_receiver(
    jobs: Arc<RwLock<HashMap<String, JobState>>>,
    id: String,
    slot_id: usize,
    sse_tx: broadcast::Sender<String>,
    mut receiver: mpsc::UnboundedReceiver<(Plan, HardSoftScore, u64)>,
) {
    let mut last_mps = 0u64;
    while let Some((solution, score, mps)) = receiver.recv().await {
        last_mps = mps;
        let _ = sse_tx.send(sse_payload(Some(score), SolverStatus::Solving, mps));
        let mut jobs = jobs.write();
        if let Some(state) = jobs.get_mut(&id) {
            state.latest = Some(solution);
            state.score = Some(score);
        }
    }
    let _ = sse_tx.send(sse_payload(
        jobs.read().get(&id).and_then(|s| s.score),
        SolverStatus::NotSolving,
        last_mps,
    ));
    let mut jobs = jobs.write();
    if let Some(state) = jobs.get_mut(&id) {
        state.status = MANAGER.get_status(slot_id);
    }
}

impl Default for SolverService {
    fn default() -> Self {
        Self::new()
    }
}
