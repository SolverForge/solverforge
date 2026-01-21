//! Value tabu acceptor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Value tabu acceptor - maintains a tabu list based on assigned values.
///
/// Unlike entity tabu which forbids recently moved entities, value tabu
/// forbids recently used values. This is useful when the problem has
/// expensive values that shouldn't be over-utilized.
pub struct ValueTabuAcceptor<S> {
    value_tabu_size: usize,
    value_tabu_list: Vec<u64>,
    current_step_values: Vec<u64>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for ValueTabuAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueTabuAcceptor")
            .field("value_tabu_size", &self.value_tabu_size)
            .field("tabu_list_len", &self.value_tabu_list.len())
            .finish()
    }
}

impl<S> Clone for ValueTabuAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            value_tabu_size: self.value_tabu_size,
            value_tabu_list: self.value_tabu_list.clone(),
            current_step_values: self.current_step_values.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S> ValueTabuAcceptor<S> {
    pub fn new(value_tabu_size: usize) -> Self {
        Self {
            value_tabu_size,
            value_tabu_list: Vec::with_capacity(value_tabu_size),
            current_step_values: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn record_value_assignment(&mut self, value_hash: u64) {
        self.current_step_values.push(value_hash);
    }

    pub fn is_value_tabu(&self, value_hash: u64) -> bool {
        self.value_tabu_list.contains(&value_hash)
    }
}

impl<S> Default for ValueTabuAcceptor<S> {
    fn default() -> Self {
        Self::new(7)
    }
}

impl<S: PlanningSolution> Acceptor<S> for ValueTabuAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        if move_score > last_step_score {
            return true;
        }
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
        let acceptor = ValueTabuAcceptor::<TestSolution>::new(5);
        assert!(!acceptor.is_value_tabu(42));
    }

    #[test]
    fn test_record_and_check() {
        let mut acceptor = ValueTabuAcceptor::<TestSolution>::new(5);
        acceptor.phase_started(&SimpleScore::of(0));

        acceptor.record_value_assignment(42);
        acceptor.step_ended(&SimpleScore::of(0));

        assert!(acceptor.is_value_tabu(42));
        assert!(!acceptor.is_value_tabu(99));
    }

    #[test]
    fn test_tabu_expiration() {
        let mut acceptor = ValueTabuAcceptor::<TestSolution>::new(2);
        acceptor.phase_started(&SimpleScore::of(0));

        acceptor.record_value_assignment(1);
        acceptor.step_ended(&SimpleScore::of(0));
        assert!(acceptor.is_value_tabu(1));

        acceptor.step_started();
        acceptor.record_value_assignment(2);
        acceptor.step_ended(&SimpleScore::of(0));
        assert!(acceptor.is_value_tabu(1));
        assert!(acceptor.is_value_tabu(2));

        acceptor.step_started();
        acceptor.record_value_assignment(3);
        acceptor.step_ended(&SimpleScore::of(0));

        assert!(!acceptor.is_value_tabu(1));
        assert!(acceptor.is_value_tabu(2));
        assert!(acceptor.is_value_tabu(3));
    }

    #[test]
    fn test_accepts_improving_move() {
        let acceptor = ValueTabuAcceptor::<TestSolution>::new(5);
        let last_score = SimpleScore::of(-10);
        let move_score = SimpleScore::of(-5);
        assert!(acceptor.is_accepted(&last_score, &move_score));
    }

    #[test]
    fn test_accepts_equal_move() {
        let acceptor = ValueTabuAcceptor::<TestSolution>::new(5);
        let score = SimpleScore::of(-10);
        assert!(acceptor.is_accepted(&score, &score));
    }

    #[test]
    fn test_rejects_worsening_move() {
        let acceptor = ValueTabuAcceptor::<TestSolution>::new(5);
        let last_score = SimpleScore::of(-5);
        let move_score = SimpleScore::of(-10);
        assert!(!acceptor.is_accepted(&last_score, &move_score));
    }

    #[test]
    fn test_phase_clears_tabu() {
        let mut acceptor = ValueTabuAcceptor::<TestSolution>::new(5);
        acceptor.phase_started(&SimpleScore::of(0));

        acceptor.record_value_assignment(42);
        acceptor.step_ended(&SimpleScore::of(0));
        assert!(acceptor.is_value_tabu(42));

        acceptor.phase_ended();
        assert!(!acceptor.is_value_tabu(42));
    }
}
