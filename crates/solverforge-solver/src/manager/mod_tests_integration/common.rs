use std::time::{Duration, Instant};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tokio::sync::mpsc::{error::TryRecvError, UnboundedReceiver};

use super::super::SolverEvent;
use crate::phase::Phase;
use crate::scope::SolverScope;

const EVENT_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn recv_event<S>(
    receiver: &mut UnboundedReceiver<SolverEvent<S>>,
    context: &str,
) -> SolverEvent<S>
where
    S: PlanningSolution,
{
    let started_at = Instant::now();
    loop {
        match receiver.try_recv() {
            Ok(event) => return event,
            Err(TryRecvError::Empty) if started_at.elapsed() < EVENT_TIMEOUT => {
                std::thread::yield_now();
            }
            Err(TryRecvError::Empty) => {
                panic!("timed out after {EVENT_TIMEOUT:?} waiting for {context}");
            }
            Err(TryRecvError::Disconnected) => {
                panic!("event stream disconnected while waiting for {context}");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct NoOpPhase;

impl<S, D, ProgressCb> Phase<S, D, ProgressCb> for NoOpPhase
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, ProgressCb>) {
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOpPhase"
    }
}
