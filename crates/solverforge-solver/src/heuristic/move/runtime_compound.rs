//! Owned compound scalar moves emitted by the one runtime-provider kernel.
//!
//! Unlike the older typed `CompoundScalarMove`, this payload owns the callback
//! reason and carries the shared static/dynamic scalar access protocol. It
//! never converts a host label into a leaked `&'static str`.

use std::fmt::{self, Debug};

#[cfg(test)]
use std::cell::Cell;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::ProviderReasonId;
use crate::builder::RuntimeScalarEdit;
use crate::stats::{CandidateTraceCoordinate, CandidateTraceIdentity};

use super::metadata::{encode_option_debug, encode_usize, hash_str, MoveTabuScope};
use super::{Move, MoveAffectedEntity, MoveTabuSignature};

#[cfg(test)]
thread_local! {
    static CLONE_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_runtime_compound_move_clone_count() {
    CLONE_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn runtime_compound_move_clone_count() -> usize {
    CLONE_COUNT.with(Cell::get)
}

/// Stable provider-family discriminator. The value is part of the exact tabu
/// identity, so identical edits emitted by distinct provider kinds cannot
/// accidentally share tabu memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RuntimeCompoundMoveKind {
    Grouped = 1,
    ConflictRepair = 2,
    CompoundConflictRepair = 3,
}

impl RuntimeCompoundMoveKind {
    pub const fn telemetry_label(self) -> &'static str {
        match self {
            Self::Grouped => "runtime_scalar_grouped",
            Self::ConflictRepair => "runtime_scalar_conflict_repair",
            Self::CompoundConflictRepair => "runtime_scalar_compound_conflict_repair",
        }
    }

    const fn trace_operation(self) -> &'static str {
        match self {
            Self::Grouped => "runtime_provider_grouped",
            Self::ConflictRepair => "runtime_provider_conflict_repair",
            Self::CompoundConflictRepair => "runtime_provider_compound_conflict_repair",
        }
    }
}

/// A selected runtime-provider result. The callback reason is a compact
/// per-run arena ID. It deliberately never participates in tabu or
/// candidate-trace identity, and candidate moves never clone/refcount labels.
pub struct RuntimeCompoundMove<S> {
    kind: RuntimeCompoundMoveKind,
    reason: ProviderReasonId,
    edits: Vec<RuntimeScalarEdit<S>>,
    entity_indices: Vec<usize>,
    descriptor_index: usize,
    require_hard_improvement: bool,
}

impl<S> Clone for RuntimeCompoundMove<S> {
    fn clone(&self) -> Self {
        #[cfg(test)]
        CLONE_COUNT.with(|count| count.set(count.get().saturating_add(1)));
        Self {
            kind: self.kind,
            reason: self.reason,
            edits: self.edits.clone(),
            entity_indices: self.entity_indices.clone(),
            descriptor_index: self.descriptor_index,
            require_hard_improvement: self.require_hard_improvement,
        }
    }
}

impl<S> RuntimeCompoundMove<S> {
    pub(crate) fn new(
        kind: RuntimeCompoundMoveKind,
        reason: ProviderReasonId,
        edits: Vec<RuntimeScalarEdit<S>>,
        require_hard_improvement: bool,
    ) -> Self {
        let descriptor_index = edits
            .first()
            .map(RuntimeScalarEdit::descriptor_index)
            .unwrap_or(usize::MAX);
        let mut entity_indices = edits
            .iter()
            .map(|edit| edit.entity_index)
            .collect::<Vec<_>>();
        entity_indices.sort_unstable();
        entity_indices.dedup();
        Self {
            kind,
            reason,
            edits,
            entity_indices,
            descriptor_index,
            require_hard_improvement,
        }
    }

    pub(crate) fn is_doable_on(&self, solution: &S) -> bool {
        if self.edits.is_empty() || has_duplicate_targets(&self.edits) {
            return false;
        }
        let mut changes_value = false;
        for edit in &self.edits {
            if edit.entity_index >= edit.slot.entity_count(solution)
                || !edit
                    .slot
                    .value_is_legal(solution, edit.entity_index, edit.to_value)
            {
                return false;
            }
            changes_value |= edit.slot.current_value(solution, edit.entity_index) != edit.to_value;
        }
        changes_value
    }

    #[cfg(test)]
    pub(crate) fn reason_id(&self) -> ProviderReasonId {
        self.reason
    }
}

impl<S> Debug for RuntimeCompoundMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeCompoundMove")
            .field("kind", &self.kind)
            .field("reason_id", &self.reason)
            .field("edits", &self.edits)
            .field("require_hard_improvement", &self.require_hard_improvement)
            .finish()
    }
}

impl<S> Move<S> for RuntimeCompoundMove<S>
where
    S: PlanningSolution,
{
    type Undo = Vec<Option<usize>>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        self.is_doable_on(score_director.working_solution())
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let mut undo = Vec::with_capacity(self.edits.len());
        let affected = unique_affected_entities(&self.edits);
        for edit in &self.edits {
            undo.push(
                edit.slot
                    .current_value(score_director.working_solution(), edit.entity_index),
            );
        }
        for (descriptor_index, entity_index) in &affected {
            score_director.before_variable_changed(*descriptor_index, *entity_index);
        }
        for edit in &self.edits {
            edit.slot.set_value(
                score_director.working_solution_mut(),
                edit.entity_index,
                edit.to_value,
            );
        }
        for (descriptor_index, entity_index) in affected.iter().rev() {
            score_director.after_variable_changed(*descriptor_index, *entity_index);
        }
        undo
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        let affected = unique_affected_entities(&self.edits);
        for (descriptor_index, entity_index) in &affected {
            score_director.before_variable_changed(*descriptor_index, *entity_index);
        }
        for (edit, old_value) in self.edits.iter().zip(undo) {
            edit.slot.set_value(
                score_director.working_solution_mut(),
                edit.entity_index,
                old_value,
            );
        }
        for (descriptor_index, entity_index) in affected.iter().rev() {
            score_director.after_variable_changed(*descriptor_index, *entity_index);
        }
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.kind.telemetry_label()
    }

    fn telemetry_label(&self) -> &'static str {
        self.kind.telemetry_label()
    }

    fn requires_hard_improvement(&self) -> bool {
        self.require_hard_improvement
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let first = self
            .edits
            .first()
            .expect("runtime compound move tabu signature requires an edit");
        let scope = MoveTabuScope::new(first.descriptor_index(), first.variable_name());
        let mut move_id = smallvec![0xDCA7_0000_0000_1000 ^ self.kind as u8 as u64];
        let mut undo_move_id = move_id.clone();
        let mut entity_tokens: SmallVec<[_; 8]> = SmallVec::new();
        let mut destination_tokens: SmallVec<[_; 8]> = SmallVec::new();

        for edit in &self.edits {
            let current = edit
                .slot
                .current_value(score_director.working_solution(), edit.entity_index);
            let descriptor = encode_usize(edit.descriptor_index());
            let variable = hash_str(edit.variable_name());
            let entity = encode_usize(edit.entity_index);
            let from = encode_option_debug(current.as_ref());
            let to = encode_option_debug(edit.to_value.as_ref());
            let edit_scope = MoveTabuScope::new(edit.descriptor_index(), edit.variable_name());

            move_id.extend([descriptor, variable, entity, from, to]);
            undo_move_id.extend([descriptor, variable, entity, to, from]);
            let entity_token = edit_scope.entity_token(entity);
            if !entity_tokens.contains(&entity_token) {
                entity_tokens.push(entity_token);
            }
            let destination_token = edit_scope.value_token(to);
            if !destination_tokens.contains(&destination_token) {
                destination_tokens.push(destination_token);
            }
        }

        MoveTabuSignature::new(scope, move_id, undo_move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens(destination_tokens)
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        Some(CandidateTraceIdentity::composite(
            self.kind.trace_operation(),
            self.edits.iter().map(|edit| {
                CandidateTraceIdentity::logical_move(
                    edit.descriptor_index(),
                    edit.variable_name(),
                    "scalar_change",
                    vec![
                        CandidateTraceCoordinate::from(edit.entity_index),
                        CandidateTraceCoordinate::from(edit.to_value),
                    ],
                )
            }),
        ))
    }

    fn for_each_affected_entity(&self, visitor: &mut dyn FnMut(MoveAffectedEntity<'_>)) {
        for edit in &self.edits {
            visitor(MoveAffectedEntity {
                descriptor_index: edit.descriptor_index(),
                entity_index: edit.entity_index,
                variable_name: edit.variable_name(),
            });
        }
    }
}

fn has_duplicate_targets<S>(edits: &[RuntimeScalarEdit<S>]) -> bool {
    let mut targets = Vec::with_capacity(edits.len());
    edits.iter().any(|edit| {
        let target = (
            edit.descriptor_index(),
            edit.variable_index(),
            edit.entity_index,
        );
        if targets.contains(&target) {
            true
        } else {
            targets.push(target);
            false
        }
    })
}

fn unique_affected_entities<S>(edits: &[RuntimeScalarEdit<S>]) -> Vec<(usize, usize)> {
    let mut affected = Vec::new();
    for edit in edits {
        let entity = (edit.descriptor_index(), edit.entity_index);
        if !affected.contains(&entity) {
            affected.push(entity);
        }
    }
    affected
}
