//! Simulated annealing acceptor.

use std::fmt::Debug;
use std::marker::PhantomData;

use rand::Rng;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::Acceptor;

/// Simulated annealing acceptor - accepts moves with temperature-based probability.
///
/// Uses the Boltzmann acceptance formula: `accept = random() < exp(-delta/T)`
/// where delta is the score difference at each level.
///
/// Temperature decays using time-gradient based cooling:
/// `T = T0 * (1 - timeGradient)` where timeGradient goes from 0 to 1.
///
/// The time_gradient must be updated externally by calling `set_time_gradient()`.
pub struct SimulatedAnnealingAcceptor<S> {
    starting_temperature: f64,
    current_temperature: f64,
    time_gradient: f64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> SimulatedAnnealingAcceptor<S> {
    /// Creates a new simulated annealing acceptor.
    ///
    /// # Arguments
    /// * `starting_temperature` - Initial temperature (higher = more exploration)
    pub fn new(starting_temperature: f64) -> Self {
        Self {
            starting_temperature,
            current_temperature: starting_temperature,
            time_gradient: 0.0,
            _phantom: PhantomData,
        }
    }

    /// Creates a simulated annealing acceptor with legacy decay rate (for backwards compatibility).
    ///
    /// The decay_rate is ignored in favor of time-gradient based cooling.
    #[deprecated(note = "Use new(starting_temperature) instead; decay_rate is ignored")]
    pub fn with_decay_rate(starting_temperature: f64, _decay_rate: f64) -> Self {
        Self::new(starting_temperature)
    }

    /// Sets the time gradient for temperature calculation.
    ///
    /// # Arguments
    /// * `time_gradient` - Progress ratio from 0.0 (start) to 1.0 (end)
    pub fn set_time_gradient(&mut self, time_gradient: f64) {
        self.time_gradient = time_gradient.clamp(0.0, 1.0);
        self.current_temperature = self.starting_temperature * (1.0 - self.time_gradient);
    }

    /// Returns the current temperature.
    pub fn current_temperature(&self) -> f64 {
        self.current_temperature
    }
}

impl<S> Default for SimulatedAnnealingAcceptor<S> {
    fn default() -> Self {
        Self::new(1.0e6) // Default high temperature for broad exploration
    }
}

impl<S> Clone for SimulatedAnnealingAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            starting_temperature: self.starting_temperature,
            current_temperature: self.current_temperature,
            time_gradient: self.time_gradient,
            _phantom: PhantomData,
        }
    }
}

impl<S> Debug for SimulatedAnnealingAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimulatedAnnealingAcceptor")
            .field("starting_temperature", &self.starting_temperature)
            .field("current_temperature", &self.current_temperature)
            .field("time_gradient", &self.time_gradient)
            .finish()
    }
}

impl<S: PlanningSolution> Acceptor<S> for SimulatedAnnealingAcceptor<S> {
    fn record_move_context(&mut self, _entity_indices: &[usize], _move_hash: u64) {
        // Simulated annealing doesn't track move context
    }

    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Always accept improving or equal moves
        if move_score >= last_step_score {
            return true;
        }

        // Reject if temperature is zero (frozen)
        if self.current_temperature <= 0.0 {
            return false;
        }

        // Boltzmann acceptance for worsening moves
        // Check each score level independently
        let last_levels = last_step_score.to_level_numbers();
        let move_levels = move_score.to_level_numbers();

        let mut rng = rand::rng();

        for (last_val, move_val) in last_levels.iter().zip(move_levels.iter()) {
            if move_val < last_val {
                // Calculate delta (positive when move is worse)
                let delta = (*last_val - *move_val) as f64;
                // Boltzmann probability: exp(-delta / T)
                let p = (-delta / self.current_temperature).exp();
                // Reject if random >= p
                if rng.random::<f64>() >= p {
                    return false;
                }
            }
        }

        // Accepted at all levels
        true
    }

    fn phase_started(&mut self, _initial_score: &S::Score) {
        self.time_gradient = 0.0;
        self.current_temperature = self.starting_temperature;
    }

    fn step_ended(&mut self, _step_score: &S::Score) {
        // Temperature is updated via set_time_gradient() called by the phase
        // This method is a no-op for time-gradient based cooling
    }

    fn set_time_gradient(&mut self, time_gradient: f64) {
        // Delegate to the inherent method
        SimulatedAnnealingAcceptor::set_time_gradient(self, time_gradient);
    }
}
