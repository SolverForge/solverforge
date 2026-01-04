//! Move tabu acceptor.

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
    /// Maximum number of moves to remember.
    move_tabu_size: usize,
    /// List of tabu move hashes.
    move_tabu_list: Vec<u64>,
    /// Current step's executed move hash.
    current_step_move: Option<u64>,
    /// Whether to accept improving moves even if tabu (aspiration).
    aspiration_enabled: bool,
    /// Best score seen so far (for aspiration criterion).
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
    /// * `move_tabu_size` - Maximum number of moves to remember as tabu
    pub fn new(move_tabu_size: usize) -> Self {
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
    pub fn without_aspiration(move_tabu_size: usize) -> Self {
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

    /// Returns true if the given move hash is in the tabu list.
    pub fn is_move_tabu(&self, move_hash: u64) -> bool {
        self.move_tabu_list.contains(&move_hash)
    }

    /// Returns true if aspiration is enabled.
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
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
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
mod tests {
    use super::*;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone)]
    struct TestSolution;
    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            None
        }
        fn set_score(&mut self, _: Option<Self::Score>) {}
    }

    #[test]
    fn test_new_acceptor() {
        let acceptor = MoveTabuAcceptor::new(5);
        assert!(!acceptor.is_move_tabu(42));
        assert!(acceptor.aspiration_enabled());
    }

    #[test]
    fn test_without_aspiration() {
        let acceptor = MoveTabuAcceptor::without_aspiration(5);
        assert!(!acceptor.aspiration_enabled());
    }

    #[test]
    fn test_record_and_check() {
        let mut acceptor = MoveTabuAcceptor::new(5);
        Acceptor::<TestSolution>::phase_started(&mut acceptor, &SimpleScore::of(0));

        acceptor.record_move(42);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));

        assert!(acceptor.is_move_tabu(42));
        assert!(!acceptor.is_move_tabu(99));
    }

    #[test]
    fn test_tabu_expiration() {
        let mut acceptor = MoveTabuAcceptor::new(2);
        Acceptor::<TestSolution>::phase_started(&mut acceptor, &SimpleScore::of(0));

        // First step: add move 1
        acceptor.record_move(1);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));
        assert!(acceptor.is_move_tabu(1));

        // Second step: add move 2
        Acceptor::<TestSolution>::step_started(&mut acceptor);
        acceptor.record_move(2);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));
        assert!(acceptor.is_move_tabu(1));
        assert!(acceptor.is_move_tabu(2));

        // Third step: add move 3 - this should evict move 1
        Acceptor::<TestSolution>::step_started(&mut acceptor);
        acceptor.record_move(3);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));

        assert!(!acceptor.is_move_tabu(1)); // Expired
        assert!(acceptor.is_move_tabu(2));
        assert!(acceptor.is_move_tabu(3));
    }

    #[test]
    fn test_accepts_improving_move() {
        let acceptor = MoveTabuAcceptor::new(5);
        let last_score = SimpleScore::of(-10);
        let move_score = SimpleScore::of(-5);
        assert!(Acceptor::<TestSolution>::is_accepted(
            &acceptor,
            &last_score,
            &move_score
        ));
    }

    #[test]
    fn test_phase_clears_tabu() {
        let mut acceptor = MoveTabuAcceptor::new(5);
        Acceptor::<TestSolution>::phase_started(&mut acceptor, &SimpleScore::of(0));

        acceptor.record_move(42);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));
        assert!(acceptor.is_move_tabu(42));

        Acceptor::<TestSolution>::phase_ended(&mut acceptor);
        assert!(!acceptor.is_move_tabu(42));
    }

    #[test]
    fn test_no_move_recorded_no_tabu() {
        let mut acceptor = MoveTabuAcceptor::new(5);
        Acceptor::<TestSolution>::phase_started(&mut acceptor, &SimpleScore::of(0));

        // Don't record any move
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));

        // No moves should be tabu
        assert!(!acceptor.is_move_tabu(42));
    }
}
