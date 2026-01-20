//! Move tabu acceptor.

use std::fmt::Debug;
use std::marker::PhantomData;

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
pub struct MoveTabuAcceptor<S> {
    move_tabu_size: usize,
    move_tabu_list: Vec<u64>,
    current_step_move: Option<u64>,
    aspiration_enabled: bool,
    best_score: Option<i64>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for MoveTabuAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoveTabuAcceptor")
            .field("move_tabu_size", &self.move_tabu_size)
            .field("tabu_list_len", &self.move_tabu_list.len())
            .field("aspiration_enabled", &self.aspiration_enabled)
            .finish()
    }
}

impl<S> Clone for MoveTabuAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            move_tabu_size: self.move_tabu_size,
            move_tabu_list: self.move_tabu_list.clone(),
            current_step_move: self.current_step_move,
            aspiration_enabled: self.aspiration_enabled,
            best_score: self.best_score,
            _phantom: PhantomData,
        }
    }
}

impl<S> MoveTabuAcceptor<S> {
    pub fn new(move_tabu_size: usize) -> Self {
        Self {
            move_tabu_size,
            move_tabu_list: Vec::with_capacity(move_tabu_size),
            current_step_move: None,
            aspiration_enabled: true,
            best_score: None,
            _phantom: PhantomData,
        }
    }

    pub fn without_aspiration(move_tabu_size: usize) -> Self {
        Self {
            move_tabu_size,
            move_tabu_list: Vec::with_capacity(move_tabu_size),
            current_step_move: None,
            aspiration_enabled: false,
            best_score: None,
            _phantom: PhantomData,
        }
    }

    pub fn record_move(&mut self, move_hash: u64) {
        self.current_step_move = Some(move_hash);
    }

    pub fn is_move_tabu(&self, move_hash: u64) -> bool {
        self.move_tabu_list.contains(&move_hash)
    }

    pub fn aspiration_enabled(&self) -> bool {
        self.aspiration_enabled
    }
}

impl<S: PlanningSolution> MoveTabuAcceptor<S> {
    fn score_to_i64(score: &S::Score) -> i64 {
        let levels = score.to_level_numbers();
        *levels.last().unwrap_or(&0)
    }
}

impl<S> Default for MoveTabuAcceptor<S> {
    fn default() -> Self {
        Self::new(7)
    }
}

impl<S: PlanningSolution> Acceptor<S> for MoveTabuAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        if self.aspiration_enabled {
            if let Some(best) = self.best_score {
                let move_value = Self::score_to_i64(move_score);
                if move_value > best {
                    return true;
                }
            }
        }
        if move_score > last_step_score {
            return true;
        }
        if move_score >= last_step_score {
            return true;
        }
        false
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.move_tabu_list.clear();
        self.current_step_move = None;
        self.best_score = Some(Self::score_to_i64(initial_score));
    }

    fn phase_ended(&mut self) {
        self.move_tabu_list.clear();
    }

    fn step_started(&mut self) {
        self.current_step_move = None;
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        if let Some(move_hash) = self.current_step_move {
            if self.move_tabu_list.len() >= self.move_tabu_size {
                self.move_tabu_list.remove(0);
            }
            self.move_tabu_list.push(move_hash);
        }
        self.current_step_move = None;

        let step_value = Self::score_to_i64(step_score);
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
        let acceptor = MoveTabuAcceptor::<TestSolution>::new(5);
        assert!(!acceptor.is_move_tabu(42));
        assert!(acceptor.aspiration_enabled());
    }

    #[test]
    fn test_without_aspiration() {
        let acceptor = MoveTabuAcceptor::<TestSolution>::without_aspiration(5);
        assert!(!acceptor.aspiration_enabled());
    }

    #[test]
    fn test_record_and_check() {
        let mut acceptor = MoveTabuAcceptor::<TestSolution>::new(5);
        acceptor.phase_started(&SimpleScore::of(0));

        acceptor.record_move(42);
        acceptor.step_ended(&SimpleScore::of(0));

        assert!(acceptor.is_move_tabu(42));
        assert!(!acceptor.is_move_tabu(99));
    }

    #[test]
    fn test_tabu_expiration() {
        let mut acceptor = MoveTabuAcceptor::<TestSolution>::new(2);
        acceptor.phase_started(&SimpleScore::of(0));

        acceptor.record_move(1);
        acceptor.step_ended(&SimpleScore::of(0));
        assert!(acceptor.is_move_tabu(1));

        acceptor.step_started();
        acceptor.record_move(2);
        acceptor.step_ended(&SimpleScore::of(0));
        assert!(acceptor.is_move_tabu(1));
        assert!(acceptor.is_move_tabu(2));

        acceptor.step_started();
        acceptor.record_move(3);
        acceptor.step_ended(&SimpleScore::of(0));

        assert!(!acceptor.is_move_tabu(1));
        assert!(acceptor.is_move_tabu(2));
        assert!(acceptor.is_move_tabu(3));
    }

    #[test]
    fn test_accepts_improving_move() {
        let acceptor = MoveTabuAcceptor::<TestSolution>::new(5);
        let last_score = SimpleScore::of(-10);
        let move_score = SimpleScore::of(-5);
        assert!(acceptor.is_accepted(&last_score, &move_score));
    }

    #[test]
    fn test_phase_clears_tabu() {
        let mut acceptor = MoveTabuAcceptor::<TestSolution>::new(5);
        acceptor.phase_started(&SimpleScore::of(0));

        acceptor.record_move(42);
        acceptor.step_ended(&SimpleScore::of(0));
        assert!(acceptor.is_move_tabu(42));

        acceptor.phase_ended();
        assert!(!acceptor.is_move_tabu(42));
    }

    #[test]
    fn test_no_move_recorded_no_tabu() {
        let mut acceptor = MoveTabuAcceptor::<TestSolution>::new(5);
        acceptor.phase_started(&SimpleScore::of(0));
        acceptor.step_ended(&SimpleScore::of(0));
        assert!(!acceptor.is_move_tabu(42));
    }
}
