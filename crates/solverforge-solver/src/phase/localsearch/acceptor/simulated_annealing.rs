//! Simulated annealing acceptor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Simulated annealing acceptor - accepts moves with temperature-based probability.
///
/// Starts with high acceptance probability and gradually decreases it,
/// allowing the search to escape local optima early on.
pub struct SimulatedAnnealingAcceptor<S> {
    starting_temperature: f64,
    current_temperature: f64,
    decay_rate: f64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> SimulatedAnnealingAcceptor<S> {
    pub fn new(starting_temperature: f64, decay_rate: f64) -> Self {
        Self {
            starting_temperature,
            current_temperature: starting_temperature,
            decay_rate,
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for SimulatedAnnealingAcceptor<S> {
    fn default() -> Self {
        Self::new(1.0, 0.99)
    }
}

impl<S> Clone for SimulatedAnnealingAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            starting_temperature: self.starting_temperature,
            current_temperature: self.current_temperature,
            decay_rate: self.decay_rate,
            _phantom: PhantomData,
        }
    }
}

impl<S> Debug for SimulatedAnnealingAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimulatedAnnealingAcceptor")
            .field("temperature", &self.current_temperature)
            .field("decay_rate", &self.decay_rate)
            .finish()
    }
}

impl<S: PlanningSolution> Acceptor<S> for SimulatedAnnealingAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        if move_score > last_step_score {
            return true;
        }
        if self.current_temperature <= 0.0 {
            return false;
        }
        let acceptance_probability = self.current_temperature.min(1.0);
        acceptance_probability > 0.5
    }

    fn phase_started(&mut self, _initial_score: &S::Score) {
        self.current_temperature = self.starting_temperature;
    }

    fn step_ended(&mut self, _step_score: &S::Score) {
        self.current_temperature *= self.decay_rate;
    }
}
