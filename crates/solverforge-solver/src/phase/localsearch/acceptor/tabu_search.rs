use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;
use crate::heuristic::r#move::{
    metadata::{MoveIdentity, ScopedEntityTabuToken, ScopedValueTabuToken},
    MoveTabuSignature,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TabuSearchPolicy {
    pub entity_tabu_size: Option<usize>,
    pub value_tabu_size: Option<usize>,
    pub move_tabu_size: Option<usize>,
    pub undo_move_tabu_size: Option<usize>,
    pub aspiration_enabled: bool,
}

impl TabuSearchPolicy {
    pub(crate) const fn move_only(move_tabu_size: usize) -> Self {
        Self {
            entity_tabu_size: None,
            value_tabu_size: None,
            move_tabu_size: Some(move_tabu_size),
            undo_move_tabu_size: None,
            aspiration_enabled: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct TabuMemory<T> {
    tenure: Option<usize>,
    entries: Vec<T>,
}

impl<T: Clone + PartialEq> TabuMemory<T> {
    fn new(tenure: Option<usize>) -> Self {
        Self {
            tenure,
            entries: Vec::new(),
        }
    }

    fn contains(&self, entry: &T) -> bool {
        self.entries.iter().any(|item| item == entry)
    }

    fn record(&mut self, entry: T) {
        let Some(tenure) = self.tenure else {
            return;
        };
        if self.entries.len() >= tenure {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    fn record_many<'a>(&mut self, entries: impl IntoIterator<Item = &'a T>)
    where
        T: 'a,
    {
        for entry in entries {
            self.record(entry.clone());
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Canonical metadata-based tabu search acceptor.
///
/// Unlike the previous score-tabu placeholder, this implementation uses the
/// actual move signature emitted by the canonical move system. The score layer
/// only participates through aspiration and phase-best tracking; admissible
/// moves are left to the forager to rank.
pub struct TabuSearchAcceptor<S: PlanningSolution> {
    entity_memory: TabuMemory<ScopedEntityTabuToken>,
    value_memory: TabuMemory<ScopedValueTabuToken>,
    move_memory: TabuMemory<MoveIdentity>,
    undo_memory: TabuMemory<MoveIdentity>,
    aspiration_enabled: bool,
    best_score: Option<S::Score>,
}

impl<S: PlanningSolution> Debug for TabuSearchAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabuSearchAcceptor")
            .field("entity_tabu_size", &self.entity_memory.tenure)
            .field("value_tabu_size", &self.value_memory.tenure)
            .field("move_tabu_size", &self.move_memory.tenure)
            .field("undo_move_tabu_size", &self.undo_memory.tenure)
            .field("aspiration_enabled", &self.aspiration_enabled)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for TabuSearchAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            entity_memory: self.entity_memory.clone(),
            value_memory: self.value_memory.clone(),
            move_memory: self.move_memory.clone(),
            undo_memory: self.undo_memory.clone(),
            aspiration_enabled: self.aspiration_enabled,
            best_score: self.best_score,
        }
    }
}

impl<S: PlanningSolution> TabuSearchAcceptor<S> {
    pub(crate) fn new(policy: TabuSearchPolicy) -> Self {
        assert!(
            policy.entity_tabu_size.is_some()
                || policy.value_tabu_size.is_some()
                || policy.move_tabu_size.is_some()
                || policy.undo_move_tabu_size.is_some(),
            "tabu_search requires at least one tabu dimension"
        );

        Self {
            entity_memory: TabuMemory::new(policy.entity_tabu_size),
            value_memory: TabuMemory::new(policy.value_tabu_size),
            move_memory: TabuMemory::new(policy.move_tabu_size),
            undo_memory: TabuMemory::new(policy.undo_move_tabu_size),
            aspiration_enabled: policy.aspiration_enabled,
            best_score: None,
        }
    }

    fn is_tabu(&self, signature: &MoveTabuSignature) -> bool {
        signature
            .entity_tokens
            .iter()
            .any(|entity_token| self.entity_memory.contains(entity_token))
            || signature
                .destination_value_tokens
                .iter()
                .any(|value_token| self.value_memory.contains(value_token))
            || self.move_memory.contains(&signature.move_id)
            || self.undo_memory.contains(&signature.undo_move_id)
    }
}

impl<S: PlanningSolution> Acceptor<S> for TabuSearchAcceptor<S> {
    fn requires_move_signatures(&self) -> bool {
        true
    }

    fn is_accepted(
        &mut self,
        _last_step_score: &S::Score,
        move_score: &S::Score,
        move_signature: Option<&MoveTabuSignature>,
    ) -> bool {
        let signature = move_signature.expect("tabu search requires move signatures");
        let aspirational = self
            .best_score
            .as_ref()
            .is_some_and(|best| self.aspiration_enabled && move_score > best);
        if !aspirational && self.is_tabu(signature) {
            return false;
        }
        true
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.entity_memory.clear();
        self.value_memory.clear();
        self.move_memory.clear();
        self.undo_memory.clear();
        self.best_score = Some(*initial_score);
    }

    fn phase_ended(&mut self) {
        self.entity_memory.clear();
        self.value_memory.clear();
        self.move_memory.clear();
        self.undo_memory.clear();
        self.best_score = None;
    }

    fn step_ended(
        &mut self,
        step_score: &S::Score,
        accepted_move_signature: Option<&MoveTabuSignature>,
    ) {
        if let Some(signature) = accepted_move_signature {
            self.entity_memory.record_many(signature.entity_tokens.iter());
            self.value_memory
                .record_many(signature.destination_value_tokens.iter());
            self.move_memory.record(signature.move_id.clone());
            self.undo_memory.record(signature.undo_move_id.clone());
        }

        if let Some(best) = &self.best_score {
            if step_score > best {
                self.best_score = Some(*step_score);
            }
        } else {
            self.best_score = Some(*step_score);
        }
    }
}
