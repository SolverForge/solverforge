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
    fn record_move_context(&mut self, _entity_indices: &[usize], move_hash: u64) {
        self.record_move(move_hash);
    }

    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Check aspiration first: accept new best even if tabu
        if self.aspiration_enabled {
            if let Some(best) = self.best_score {
                let move_value = Self::score_to_i64(move_score);
                if move_value > best {
                    return true;
                }
            }
        }

        // Check if current move is tabu - reject if so
        if let Some(move_hash) = self.current_step_move {
            if self.is_move_tabu(move_hash) {
                return false;
            }
        }

        // Normal acceptance: accept improving or equal moves
        move_score >= last_step_score
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
