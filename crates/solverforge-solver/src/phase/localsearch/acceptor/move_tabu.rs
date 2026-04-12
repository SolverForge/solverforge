// Move tabu acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::Score;

use super::Acceptor;

/// Move tabu acceptor - maintains a tabu list based on move identifiers.
///
/// Unlike entity tabu (which forbids recently moved entities) or value tabu
/// (which forbids recently assigned values), move tabu forbids the exact
/// move combination (entity + value). This provides finer-grained control.
///
/// A move is identified by its hash, typically combining entity index and
/// assigned value information.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::MoveTabuAcceptor;
///
/// let acceptor = MoveTabuAcceptor::new(7);
/// assert!(!acceptor.is_move_tabu(42));
/// ```
pub struct MoveTabuAcceptor {
    // Maximum number of moves to remember.
    move_tabu_size: usize,
    // List of tabu move hashes.
    move_tabu_list: Vec<u64>,
    // Current step's executed move hash.
    current_step_move: Option<u64>,
    // Whether to accept improving moves even if tabu (aspiration).
    aspiration_enabled: bool,
    // Best score seen so far (for aspiration criterion).
    best_score: Option<i64>,
}

impl Debug for MoveTabuAcceptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoveTabuAcceptor")
            .field("move_tabu_size", &self.move_tabu_size)
            .field("tabu_list_len", &self.move_tabu_list.len())
            .field("aspiration_enabled", &self.aspiration_enabled)
            .finish()
    }
}

impl Clone for MoveTabuAcceptor {
    fn clone(&self) -> Self {
        Self {
            move_tabu_size: self.move_tabu_size,
            move_tabu_list: self.move_tabu_list.clone(),
            current_step_move: self.current_step_move,
            aspiration_enabled: self.aspiration_enabled,
            best_score: self.best_score,
        }
    }
}

impl MoveTabuAcceptor {
    /// Creates a new move tabu acceptor with aspiration enabled.
    ///
    /// # Arguments
    /// * `move_tabu_size` - Maximum number of moves to remember as tabu. Must be > 0.
    ///
    /// # Panics
    ///
    /// Panics if `move_tabu_size` is 0.
    pub fn new(move_tabu_size: usize) -> Self {
        assert!(move_tabu_size > 0, "move_tabu_size must be > 0, got 0");
        Self {
            move_tabu_size,
            move_tabu_list: Vec::with_capacity(move_tabu_size),
            current_step_move: None,
            aspiration_enabled: true,
            best_score: None,
        }
    }

    /// Creates a move tabu acceptor without aspiration.
    ///
    /// Without aspiration, tabu moves are never accepted even if they
    /// would lead to a new best solution.
    ///
    /// # Panics
    ///
    /// Panics if `move_tabu_size` is 0.
    pub fn without_aspiration(move_tabu_size: usize) -> Self {
        assert!(move_tabu_size > 0, "move_tabu_size must be > 0, got 0");
        Self {
            move_tabu_size,
            move_tabu_list: Vec::with_capacity(move_tabu_size),
            current_step_move: None,
            aspiration_enabled: false,
            best_score: None,
        }
    }

    /// Records that a move was executed in the current step.
    ///
    /// Call this with the hash of the executed move.
    pub fn record_move(&mut self, move_hash: u64) {
        self.current_step_move = Some(move_hash);
    }

    pub fn is_move_tabu(&self, move_hash: u64) -> bool {
        self.move_tabu_list.contains(&move_hash)
    }

    pub fn aspiration_enabled(&self) -> bool {
        self.aspiration_enabled
    }

    fn score_to_i64<S: PlanningSolution>(score: &S::Score) -> i64 {
        let levels = score.to_level_numbers();
        // Use last level (soft score) as the primary comparison
        *levels.last().unwrap_or(&0)
    }
}

impl Default for MoveTabuAcceptor {
    fn default() -> Self {
        Self::new(7)
    }
}

impl<S: PlanningSolution> Acceptor<S> for MoveTabuAcceptor {
    fn is_accepted(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Check aspiration criterion
        if self.aspiration_enabled {
            if let Some(best) = self.best_score {
                let move_value = Self::score_to_i64::<S>(move_score);
                if move_value > best {
                    return true; // Aspiration: accept new best even if tabu
                }
            }
        }

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

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.move_tabu_list.clear();
        self.current_step_move = None;
        self.best_score = Some(Self::score_to_i64::<S>(initial_score));
    }

    fn phase_ended(&mut self) {
        self.move_tabu_list.clear();
    }

    fn step_started(&mut self) {
        self.current_step_move = None;
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        // Add current step's move to tabu list
        if let Some(move_hash) = self.current_step_move {
            if self.move_tabu_list.len() >= self.move_tabu_size {
                self.move_tabu_list.remove(0);
            }
            self.move_tabu_list.push(move_hash);
        }
        self.current_step_move = None;

        // Update best score
        let step_value = Self::score_to_i64::<S>(step_score);
        if let Some(best) = self.best_score {
            if step_value > best {
                self.best_score = Some(step_value);
            }
        } else {
            self.best_score = Some(step_value);
        }
    }
}

#[cfg(test)]
#[path = "move_tabu_tests.rs"]
mod tests;
