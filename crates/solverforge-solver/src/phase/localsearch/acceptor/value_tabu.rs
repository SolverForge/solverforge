//! Value tabu acceptor.

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
    /// Maximum number of values to remember.
    value_tabu_size: usize,
    /// List of tabu value hashes.
    value_tabu_list: Vec<u64>,
    /// Current step's assigned values.
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
    /// * `value_tabu_size` - Maximum number of values to remember as tabu
    pub fn new(value_tabu_size: usize) -> Self {
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

    /// Returns true if the given value hash is in the tabu list.
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
        let acceptor = ValueTabuAcceptor::new(5);
        assert!(!acceptor.is_value_tabu(42));
    }

    #[test]
    fn test_record_and_check() {
        let mut acceptor = ValueTabuAcceptor::new(5);
        Acceptor::<TestSolution>::phase_started(&mut acceptor, &SimpleScore::of(0));

        acceptor.record_value_assignment(42);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));

        assert!(acceptor.is_value_tabu(42));
        assert!(!acceptor.is_value_tabu(99));
    }

    #[test]
    fn test_tabu_expiration() {
        let mut acceptor = ValueTabuAcceptor::new(2);
        Acceptor::<TestSolution>::phase_started(&mut acceptor, &SimpleScore::of(0));

        // First step: add value 1
        acceptor.record_value_assignment(1);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));
        assert!(acceptor.is_value_tabu(1));

        // Second step: add value 2
        Acceptor::<TestSolution>::step_started(&mut acceptor);
        acceptor.record_value_assignment(2);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));
        assert!(acceptor.is_value_tabu(1));
        assert!(acceptor.is_value_tabu(2));

        // Third step: add value 3 - this should evict value 1
        Acceptor::<TestSolution>::step_started(&mut acceptor);
        acceptor.record_value_assignment(3);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));

        assert!(!acceptor.is_value_tabu(1)); // Expired
        assert!(acceptor.is_value_tabu(2));
        assert!(acceptor.is_value_tabu(3));
    }

    #[test]
    fn test_accepts_improving_move() {
        let mut acceptor = ValueTabuAcceptor::new(5);
        let last_score = SimpleScore::of(-10);
        let move_score = SimpleScore::of(-5);
        assert!(Acceptor::<TestSolution>::is_accepted(
            &mut acceptor,
            &last_score,
            &move_score
        ));
    }

    #[test]
    fn test_accepts_equal_move() {
        let mut acceptor = ValueTabuAcceptor::new(5);
        let score = SimpleScore::of(-10);
        assert!(Acceptor::<TestSolution>::is_accepted(
            &mut acceptor,
            &score,
            &score
        ));
    }

    #[test]
    fn test_rejects_worsening_move() {
        let mut acceptor = ValueTabuAcceptor::new(5);
        let last_score = SimpleScore::of(-5);
        let move_score = SimpleScore::of(-10);
        assert!(!Acceptor::<TestSolution>::is_accepted(
            &mut acceptor,
            &last_score,
            &move_score
        ));
    }

    #[test]
    fn test_phase_clears_tabu() {
        let mut acceptor = ValueTabuAcceptor::new(5);
        Acceptor::<TestSolution>::phase_started(&mut acceptor, &SimpleScore::of(0));

        acceptor.record_value_assignment(42);
        Acceptor::<TestSolution>::step_ended(&mut acceptor, &SimpleScore::of(0));
        assert!(acceptor.is_value_tabu(42));

        Acceptor::<TestSolution>::phase_ended(&mut acceptor);
        assert!(!acceptor.is_value_tabu(42));
    }
}
