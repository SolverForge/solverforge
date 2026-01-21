//! Entity tabu acceptor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Entity tabu acceptor - maintains a tabu list based on entity identifiers.
///
/// This is a more sophisticated tabu search that tracks which entities
/// have been recently modified rather than just tracking scores.
/// It requires entities to have a stable hash/identifier.
pub struct EntityTabuAcceptor<S> {
    entity_tabu_size: usize,
    entity_tabu_list: Vec<u64>,
    current_step_entities: Vec<u64>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for EntityTabuAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityTabuAcceptor")
            .field("entity_tabu_size", &self.entity_tabu_size)
            .field("tabu_list_len", &self.entity_tabu_list.len())
            .finish()
    }
}

impl<S> Clone for EntityTabuAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            entity_tabu_size: self.entity_tabu_size,
            entity_tabu_list: self.entity_tabu_list.clone(),
            current_step_entities: self.current_step_entities.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S> EntityTabuAcceptor<S> {
    pub fn new(entity_tabu_size: usize) -> Self {
        Self {
            entity_tabu_size,
            entity_tabu_list: Vec::with_capacity(entity_tabu_size),
            current_step_entities: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn record_entity_move(&mut self, entity_id: u64) {
        self.current_step_entities.push(entity_id);
    }

    pub fn is_entity_tabu(&self, entity_id: u64) -> bool {
        self.entity_tabu_list.contains(&entity_id)
    }
}

impl<S> Default for EntityTabuAcceptor<S> {
    fn default() -> Self {
        Self::new(7)
    }
}

impl<S: PlanningSolution> Acceptor<S> for EntityTabuAcceptor<S> {
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
        self.entity_tabu_list.clear();
        self.current_step_entities.clear();
    }

    fn phase_ended(&mut self) {
        self.entity_tabu_list.clear();
    }

    fn step_started(&mut self) {
        self.current_step_entities.clear();
    }

    fn step_ended(&mut self, _step_score: &S::Score) {
        for entity_id in &self.current_step_entities {
            if self.entity_tabu_list.len() >= self.entity_tabu_size {
                self.entity_tabu_list.remove(0);
            }
            self.entity_tabu_list.push(*entity_id);
        }
        self.current_step_entities.clear();
    }
}
