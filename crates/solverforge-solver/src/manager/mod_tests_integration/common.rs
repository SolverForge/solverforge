use std::time::{Duration, Instant};

use solverforge_core::domain::PlanningSolution;
use tokio::sync::mpsc::{error::TryRecvError, UnboundedReceiver};

use super::super::SolverEvent;

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
