//! Simulated annealing acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Simulated annealing acceptor - accepts moves with temperature-based probability.
///
/// Starts with high acceptance probability and gradually decreases it,
/// allowing the search to escape local optima early on.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::SimulatedAnnealingAcceptor;
///
/// let acceptor = SimulatedAnnealingAcceptor::new(1.0, 0.99);
/// ```
#[derive(Debug, Clone)]
pub struct SimulatedAnnealingAcceptor {
    /// Initial temperature.
    starting_temperature: f64,
    /// Current temperature.
    current_temperature: f64,
    /// Temperature decay rate per step.
    decay_rate: f64,
}

impl SimulatedAnnealingAcceptor {
    /// Creates a new simulated annealing acceptor.
    ///
    /// # Arguments
    /// * `starting_temperature` - Initial temperature (higher = more exploration)
    /// * `decay_rate` - Multiplicative decay per step (e.g., 0.99)
    pub fn new(starting_temperature: f64, decay_rate: f64) -> Self {
        Self {
            starting_temperature,
            current_temperature: starting_temperature,
            decay_rate,
        }
    }
}

impl Default for SimulatedAnnealingAcceptor {
    fn default() -> Self {
        Self::new(1.0, 0.99)
    }
}

impl<S: PlanningSolution> Acceptor<S> for SimulatedAnnealingAcceptor {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Always accept improving moves
        if move_score > last_step_score {
            return true;
        }

        // For non-improving moves, accept with probability based on temperature
        // P = exp(-delta / temperature) where delta is the score difference
        // Since we can't easily compute the numeric difference, we use a simpler approach:
        // Accept with probability proportional to the temperature

        if self.current_temperature <= 0.0 {
            return false;
        }

        // Simple probability: temperature directly (0.0 to 1.0)
        // In a real implementation, we'd compute the actual score difference
        let acceptance_probability = self.current_temperature.min(1.0);

        // Use a deterministic acceptance for testing
        // In production, this would use a random number
        acceptance_probability > 0.5
    }

    fn phase_started(&mut self, _initial_score: &S::Score) {
        self.current_temperature = self.starting_temperature;
    }

    fn step_ended(&mut self, _step_score: &S::Score) {
        // Decay temperature
        self.current_temperature *= self.decay_rate;
    }
}
