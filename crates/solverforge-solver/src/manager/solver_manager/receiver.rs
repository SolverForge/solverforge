use std::error::Error;
use std::fmt::{self, Debug, Display};

use solverforge_core::domain::PlanningSolution;
use tokio::sync::mpsc::{self, error::TryRecvError};

use super::types::SolverEvent;

/// The owned stream of lifecycle events for one retained solve job.
///
/// [`SolverManager::solve`](crate::SolverManager::solve) returns one receiver
/// per job. Events arrive in publication order; the stream ends once the job
/// is deleted and its buffered events are drained.
///
/// The receiver is unbounded: the solver never blocks on a slow consumer.
pub struct SolverEventReceiver<S: PlanningSolution> {
    receiver: mpsc::UnboundedReceiver<SolverEvent<S>>,
}

impl<S: PlanningSolution> SolverEventReceiver<S> {
    pub(crate) fn new(receiver: mpsc::UnboundedReceiver<SolverEvent<S>>) -> Self {
        Self { receiver }
    }

    /// Waits for the next event.
    ///
    /// Returns `None` once the job's sender is gone and every buffered event
    /// has been received.
    pub async fn recv(&mut self) -> Option<SolverEvent<S>> {
        self.receiver.recv().await
    }

    /// Blocks the current thread until the next event arrives.
    ///
    /// Returns `None` once the job's sender is gone and every buffered event
    /// has been received.
    ///
    /// # Panics
    ///
    /// Panics when called inside an asynchronous execution context; use
    /// [`Self::recv`] there.
    pub fn blocking_recv(&mut self) -> Option<SolverEvent<S>> {
        self.receiver.blocking_recv()
    }

    /// Returns the next buffered event without waiting.
    pub fn try_recv(&mut self) -> Result<SolverEvent<S>, SolverEventTryRecvError> {
        self.receiver.try_recv().map_err(|error| match error {
            TryRecvError::Empty => SolverEventTryRecvError::Empty,
            TryRecvError::Disconnected => SolverEventTryRecvError::Disconnected,
        })
    }
}

impl<S: PlanningSolution> Debug for SolverEventReceiver<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SolverEventReceiver")
            .field("buffered", &self.receiver.len())
            .field("closed", &self.receiver.is_closed())
            .finish()
    }
}

/// Why [`SolverEventReceiver::try_recv`] returned no event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverEventTryRecvError {
    /// No event is buffered yet; the job can still publish more.
    Empty,
    /// The job's sender is gone and every buffered event has been received.
    Disconnected,
}

impl Display for SolverEventTryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("no solver event is buffered"),
            Self::Disconnected => f.write_str("the solver event stream is disconnected"),
        }
    }
}

impl Error for SolverEventTryRecvError {}
