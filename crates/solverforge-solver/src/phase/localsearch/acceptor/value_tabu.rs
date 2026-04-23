// Value tabu acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Value tabu acceptor - maintains a tabu list based on assigned values.
///
/// Unlike entity tabu which forbids recently moved entities, value tabu
/// forbids recently used values. This is useful when the problem has
/// expensive values that shouldn't be over-utilized.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::ValueTabuAcceptor;
///
/// let acceptor = ValueTabuAcceptor::new(7);
/// assert!(!acceptor.is_value_tabu(42));
/// ```
pub struct ValueTabuAcceptor {
    // Maximum number of values to remember.
    value_tabu_size: usize,
    // List of tabu value hashes.
    value_tabu_list: Vec<u64>,
    // Current step's assigned values.
    current_step_values: Vec<u64>,
}

impl Debug for ValueTabuAcceptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueTabuAcceptor")
            .field("value_tabu_size", &self.value_tabu_size)
            .field("tabu_list_len", &self.value_tabu_list.len())
            .finish()
    }
}

impl Clone for ValueTabuAcceptor {
    fn clone(&self) -> Self {
        Self {
            value_tabu_size: self.value_tabu_size,
            value_tabu_list: self.value_tabu_list.clone(),
            current_step_values: self.current_step_values.clone(),
        }
    }
}

impl ValueTabuAcceptor {
    /// Creates a new value tabu acceptor.
    ///
    /// # Arguments
    /// * `value_tabu_size` - Maximum number of values to remember as tabu. Must be > 0.
    ///
    /// # Panics
    ///
    /// Panics if `value_tabu_size` is 0.
    pub fn new(value_tabu_size: usize) -> Self {
        assert!(value_tabu_size > 0, "value_tabu_size must be > 0, got 0");
        Self {
            value_tabu_size,
            value_tabu_list: Vec::with_capacity(value_tabu_size),
            current_step_values: Vec::new(),
        }
    }

    /// Records that a value was assigned in the current step.
    ///
    /// Call this with the hash of the assigned value before accepting the move.
    pub fn record_value_assignment(&mut self, value_hash: u64) {
        self.current_step_values.push(value_hash);
    }

    pub fn is_value_tabu(&self, value_hash: u64) -> bool {
        self.value_tabu_list.contains(&value_hash)
    }
}

impl Default for ValueTabuAcceptor {
    fn default() -> Self {
        Self::new(7)
    }
}

impl<S: PlanningSolution> Acceptor<S> for ValueTabuAcceptor {
    fn is_accepted(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Accept improving moves
        if move_score > last_step_score {
            return true;
        }

        // Accept equal moves for plateau exploration
        if move_score >= last_step_score {
            return true;
        }

        false
    }

    fn phase_started(&mut self, _initial_score: &S::Score) {
        self.value_tabu_list.clear();
        self.current_step_values.clear();
    }

    fn phase_ended(&mut self) {
        self.value_tabu_list.clear();
    }

    fn step_started(&mut self) {
        self.current_step_values.clear();
    }

    fn step_ended(&mut self, _step_score: &S::Score) {
        // Add current step's values to tabu list
        for value_hash in &self.current_step_values {
            if self.value_tabu_list.len() >= self.value_tabu_size {
                self.value_tabu_list.remove(0);
            }
            self.value_tabu_list.push(*value_hash);
        }
        self.current_step_values.clear();
    }
}

#[cfg(test)]
mod tests;
