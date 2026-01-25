//! Value tabu acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Value tabu acceptor - maintains a tabu list based on assigned values.
///
/// Unlike entity tabu which forbids recently moved entities, value tabu
/// forbids recently used values. This is useful when the problem has
/// expensive values that shouldn't be over-utilized.
pub struct ValueTabuAcceptor<S: PlanningSolution> {
    value_tabu_size: usize,
    value_tabu_list: Vec<u64>,
    current_step_values: Vec<u64>,
    aspiration_enabled: bool,
    best_score: Option<S::Score>,
}

impl<S: PlanningSolution> Debug for ValueTabuAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueTabuAcceptor")
            .field("value_tabu_size", &self.value_tabu_size)
            .field("tabu_list_len", &self.value_tabu_list.len())
            .field("aspiration_enabled", &self.aspiration_enabled)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for ValueTabuAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            value_tabu_size: self.value_tabu_size,
            value_tabu_list: self.value_tabu_list.clone(),
            current_step_values: self.current_step_values.clone(),
            aspiration_enabled: self.aspiration_enabled,
            best_score: self.best_score,
        }
    }
}

impl<S: PlanningSolution> ValueTabuAcceptor<S> {
    pub fn new(value_tabu_size: usize) -> Self {
        Self {
            value_tabu_size,
            value_tabu_list: Vec::with_capacity(value_tabu_size),
            current_step_values: Vec::new(),
            aspiration_enabled: true,
            best_score: None,
        }
    }

    pub fn without_aspiration(value_tabu_size: usize) -> Self {
        Self {
            value_tabu_size,
            value_tabu_list: Vec::with_capacity(value_tabu_size),
            current_step_values: Vec::new(),
            aspiration_enabled: false,
            best_score: None,
        }
    }

    pub fn record_value_assignment(&mut self, value_hash: u64) {
        self.current_step_values.push(value_hash);
    }

    pub fn is_value_tabu(&self, value_hash: u64) -> bool {
        self.value_tabu_list.contains(&value_hash)
    }

    pub fn aspiration_enabled(&self) -> bool {
        self.aspiration_enabled
    }
}

impl<S: PlanningSolution> Default for ValueTabuAcceptor<S> {
    fn default() -> Self {
        Self::new(7)
    }
}

impl<S: PlanningSolution> Acceptor<S> for ValueTabuAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Check aspiration first: accept new best even if tabu
        if self.aspiration_enabled {
            if let Some(ref best) = self.best_score {
                if move_score > best {
                    return true;
                }
            }
        }

        // Check if any value in current move is tabu - reject if so
        for value_hash in &self.current_step_values {
            if self.is_value_tabu(*value_hash) {
                return false;
            }
        }

        // Normal acceptance: accept improving or equal moves
        move_score >= last_step_score
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.value_tabu_list.clear();
        self.current_step_values.clear();
        self.best_score = Some(*initial_score);
    }

    fn phase_ended(&mut self) {
        self.value_tabu_list.clear();
    }

    fn step_started(&mut self) {
        self.current_step_values.clear();
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        for value_hash in &self.current_step_values {
            if self.value_tabu_list.len() >= self.value_tabu_size {
                self.value_tabu_list.remove(0);
            }
            self.value_tabu_list.push(*value_hash);
        }
        self.current_step_values.clear();

        // Update best score
        if let Some(ref best) = self.best_score {
            if step_score > best {
                self.best_score = Some(*step_score);
            }
        } else {
            self.best_score = Some(*step_score);
        }
    }
}
