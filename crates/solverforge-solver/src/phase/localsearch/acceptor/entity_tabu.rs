//! Entity tabu acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Entity tabu acceptor - maintains a tabu list based on entity identifiers.
///
/// This is a more sophisticated tabu search that tracks which entities
/// have been recently modified rather than just tracking scores.
/// It requires entities to have a stable hash/identifier.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::EntityTabuAcceptor;
///
/// let acceptor = EntityTabuAcceptor::new(7);
/// assert!(!acceptor.is_entity_tabu(42));
/// ```
pub struct EntityTabuAcceptor {
    /// Maximum number of entity changes to remember.
    entity_tabu_size: usize,
    /// List of tabu entity identifiers (as u64 hashes).
    entity_tabu_list: Vec<u64>,
    /// Current step's moved entities.
    current_step_entities: Vec<u64>,
}

impl Debug for EntityTabuAcceptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityTabuAcceptor")
            .field("entity_tabu_size", &self.entity_tabu_size)
            .field("tabu_list_len", &self.entity_tabu_list.len())
            .finish()
    }
}

impl Clone for EntityTabuAcceptor {
    fn clone(&self) -> Self {
        Self {
            entity_tabu_size: self.entity_tabu_size,
            entity_tabu_list: self.entity_tabu_list.clone(),
            current_step_entities: self.current_step_entities.clone(),
        }
    }
}

impl EntityTabuAcceptor {
    /// Creates a new entity tabu acceptor.
    ///
    /// # Panics
    ///
    /// Panics if `entity_tabu_size` is 0.
    pub fn new(entity_tabu_size: usize) -> Self {
        assert!(entity_tabu_size > 0, "entity_tabu_size must be > 0, got 0");
        Self {
            entity_tabu_size,
            entity_tabu_list: Vec::with_capacity(entity_tabu_size),
            current_step_entities: Vec::new(),
        }
    }

    /// Records that an entity was moved in the current step.
    pub fn record_entity_move(&mut self, entity_id: u64) {
        self.current_step_entities.push(entity_id);
    }

    /// Returns true if the given entity is in the tabu list.
    pub fn is_entity_tabu(&self, entity_id: u64) -> bool {
        self.entity_tabu_list.contains(&entity_id)
    }
}

impl Default for EntityTabuAcceptor {
    fn default() -> Self {
        Self::new(7)
    }
}

impl<S: PlanningSolution> Acceptor<S> for EntityTabuAcceptor {
    fn is_accepted(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Accept improving moves
        if move_score > last_step_score {
            return true;
        }

        // Accept equal moves for exploration
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
        // Add current step's entities to tabu list
        for entity_id in &self.current_step_entities {
            if self.entity_tabu_list.len() >= self.entity_tabu_size {
                self.entity_tabu_list.remove(0);
            }
            self.entity_tabu_list.push(*entity_id);
        }
        self.current_step_entities.clear();
    }
}
