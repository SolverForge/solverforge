//! Entity tabu acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Entity tabu acceptor - maintains a tabu list based on entity identifiers.
///
/// This is a more sophisticated tabu search that tracks which entities
/// have been recently modified rather than just tracking scores.
/// It requires entities to have a stable hash/identifier.
pub struct EntityTabuAcceptor<S: PlanningSolution> {
    entity_tabu_size: usize,
    entity_tabu_list: Vec<u64>,
    current_step_entities: Vec<u64>,
    aspiration_enabled: bool,
    best_score: Option<S::Score>,
}

impl<S: PlanningSolution> Debug for EntityTabuAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityTabuAcceptor")
            .field("entity_tabu_size", &self.entity_tabu_size)
            .field("tabu_list_len", &self.entity_tabu_list.len())
            .field("aspiration_enabled", &self.aspiration_enabled)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for EntityTabuAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            entity_tabu_size: self.entity_tabu_size,
            entity_tabu_list: self.entity_tabu_list.clone(),
            current_step_entities: self.current_step_entities.clone(),
            aspiration_enabled: self.aspiration_enabled,
            best_score: self.best_score,
        }
    }
}

impl<S: PlanningSolution> EntityTabuAcceptor<S> {
    pub fn new(entity_tabu_size: usize) -> Self {
        Self {
            entity_tabu_size,
            entity_tabu_list: Vec::with_capacity(entity_tabu_size),
            current_step_entities: Vec::new(),
            aspiration_enabled: true,
            best_score: None,
        }
    }

    pub fn without_aspiration(entity_tabu_size: usize) -> Self {
        Self {
            entity_tabu_size,
            entity_tabu_list: Vec::with_capacity(entity_tabu_size),
            current_step_entities: Vec::new(),
            aspiration_enabled: false,
            best_score: None,
        }
    }

    pub fn record_entity_move(&mut self, entity_id: u64) {
        self.current_step_entities.push(entity_id);
    }

    pub fn is_entity_tabu(&self, entity_id: u64) -> bool {
        self.entity_tabu_list.contains(&entity_id)
    }

    pub fn aspiration_enabled(&self) -> bool {
        self.aspiration_enabled
    }
}

impl<S: PlanningSolution> Default for EntityTabuAcceptor<S> {
    fn default() -> Self {
        Self::new(7)
    }
}

impl<S: PlanningSolution> Acceptor<S> for EntityTabuAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Check aspiration first: accept new best even if tabu
        if self.aspiration_enabled {
            if let Some(ref best) = self.best_score {
                if move_score > best {
                    return true;
                }
            }
        }

        // Check if any entity in current move is tabu - reject if so
        for entity_id in &self.current_step_entities {
            if self.is_entity_tabu(*entity_id) {
                return false;
            }
        }

        // Normal acceptance: accept improving or equal moves
        move_score >= last_step_score
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.entity_tabu_list.clear();
        self.current_step_entities.clear();
        self.best_score = Some(*initial_score);
    }

    fn phase_ended(&mut self) {
        self.entity_tabu_list.clear();
    }

    fn step_started(&mut self) {
        self.current_step_entities.clear();
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        for entity_id in &self.current_step_entities {
            if self.entity_tabu_list.len() >= self.entity_tabu_size {
                self.entity_tabu_list.remove(0);
            }
            self.entity_tabu_list.push(*entity_id);
        }
        self.current_step_entities.clear();

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
