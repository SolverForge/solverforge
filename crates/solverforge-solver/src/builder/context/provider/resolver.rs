use std::collections::HashSet;
use std::sync::Arc;

use super::super::{RuntimeScalarSlot, RuntimeScalarSlotId};
use super::types::ProviderCandidateReason;
use super::{
    ProviderNormalizationState, ProviderReasonArena, ProviderResolutionError, RawProviderCandidate,
    RawProviderEdit, ResolvedProviderCandidate, ResolvedProviderEdit,
};
use crate::{RepairCandidate, ScalarCandidate, ScalarEdit};

#[derive(Debug)]
struct RuntimeProviderSlot<S> {
    id: RuntimeScalarSlotId,
    slot: RuntimeScalarSlot<S>,
}

trait ProviderEditAccess {
    fn entity_index(&self) -> usize;
    fn to_value(&self) -> Option<usize>;
}

impl ProviderEditAccess for RawProviderEdit {
    fn entity_index(&self) -> usize {
        self.entity_index
    }

    fn to_value(&self) -> Option<usize> {
        self.to_value
    }
}

impl<S> ProviderEditAccess for ScalarEdit<S> {
    fn entity_index(&self) -> usize {
        self.entity_index()
    }

    fn to_value(&self) -> Option<usize> {
        self.to_value()
    }
}

impl<S> Clone for RuntimeProviderSlot<S> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            slot: self.slot.clone(),
        }
    }
}

/// Canonical first-match resolver for raw callback names.
///
/// Slot order is exactly `RuntimeModel.variables()` scalar order. An
/// unqualified edit therefore resolves in the allowed model universe rather
/// than by scanning mutable host schema state.
#[derive(Debug)]
pub struct RuntimeProviderSlotResolver<S> {
    slots: Vec<RuntimeProviderSlot<S>>,
}

impl<S> Clone for RuntimeProviderSlotResolver<S> {
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
        }
    }
}

impl<S> RuntimeProviderSlotResolver<S> {
    pub fn new(slots: Vec<RuntimeScalarSlot<S>>) -> Result<Self, String> {
        let mut seen = HashSet::new();
        let mut resolved = Vec::with_capacity(slots.len());
        for slot in slots {
            let id = slot.id();
            if !seen.insert((id.descriptor_index, id.variable_index)) {
                return Err(format!(
                    "runtime provider registry has duplicate scalar slot {}.{}",
                    id.entity_class, id.variable_name
                ));
            }
            resolved.push(RuntimeProviderSlot { id, slot });
        }
        Ok(Self { slots: resolved })
    }

    pub fn slot_ids(&self) -> impl Iterator<Item = &RuntimeScalarSlotId> {
        self.slots.iter().map(|slot| &slot.id)
    }

    fn resolve_raw_index(
        &self,
        edit: &RawProviderEdit,
        allowed_slots: &[RuntimeScalarSlotId],
    ) -> Result<usize, ProviderResolutionError> {
        let matches = |slot: &RuntimeProviderSlot<S>| {
            edit.entity_class
                .as_deref()
                .is_none_or(|entity| entity == slot.id.entity_class.as_ref())
                && edit.variable_name.as_ref() == slot.id.variable_name.as_ref()
        };
        let Some(first_matching_index) = self.slots.iter().position(matches) else {
            return Err(ProviderResolutionError::UnknownSlot {
                entity_class: edit.entity_class.clone(),
                variable_name: Arc::clone(&edit.variable_name),
            });
        };
        let is_allowed = |slot: &RuntimeProviderSlot<S>| {
            allowed_slots.iter().any(|candidate| {
                candidate.descriptor_index == slot.id.descriptor_index
                    && candidate.variable_index == slot.id.variable_index
            })
        };
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| matches(slot) && is_allowed(slot))
        else {
            let first_matching_slot = &self.slots[first_matching_index];
            return Err(ProviderResolutionError::SlotOutsideSelector {
                entity_class: Arc::clone(&first_matching_slot.id.entity_class),
                variable_name: Arc::clone(&first_matching_slot.id.variable_name),
            });
        };
        Ok(index)
    }

    fn resolve_static_index(
        &self,
        edit: &ScalarEdit<S>,
        allowed_slots: &[RuntimeScalarSlotId],
    ) -> Result<usize, ProviderResolutionError> {
        let matches = |slot: &RuntimeProviderSlot<S>| {
            slot.id.descriptor_index == edit.descriptor_index()
                && slot.id.variable_name.as_ref() == edit.variable_name()
        };
        let Some(first_matching_index) = self.slots.iter().position(matches) else {
            return Err(ProviderResolutionError::UnknownSlot {
                entity_class: None,
                variable_name: Arc::from(edit.variable_name()),
            });
        };
        let is_allowed = |slot: &RuntimeProviderSlot<S>| {
            allowed_slots.iter().any(|candidate| {
                candidate.descriptor_index == slot.id.descriptor_index
                    && candidate.variable_index == slot.id.variable_index
            })
        };
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| matches(slot) && is_allowed(slot))
        else {
            let first_matching_slot = &self.slots[first_matching_index];
            return Err(ProviderResolutionError::SlotOutsideSelector {
                entity_class: Arc::clone(&first_matching_slot.id.entity_class),
                variable_name: Arc::clone(&first_matching_slot.id.variable_name),
            });
        };
        Ok(index)
    }

    pub fn resolve_and_normalize(
        &self,
        solution: &S,
        candidates: Vec<RawProviderCandidate>,
        allowed_slots: &[RuntimeScalarSlotId],
        reasons: &mut ProviderReasonArena,
    ) -> Result<Vec<ResolvedProviderCandidate<S>>, ProviderResolutionError> {
        self.resolve_and_normalize_with_state(
            solution,
            candidates,
            allowed_slots,
            &mut ProviderNormalizationState::default(),
            reasons,
        )
    }

    /// Resolves raw candidates in returned order while applying deduplication
    /// in the explicit caller-owned scope.
    pub fn resolve_and_normalize_with_state(
        &self,
        solution: &S,
        candidates: Vec<RawProviderCandidate>,
        allowed_slots: &[RuntimeScalarSlotId],
        state: &mut ProviderNormalizationState,
        reasons: &mut ProviderReasonArena,
    ) -> Result<Vec<ResolvedProviderCandidate<S>>, ProviderResolutionError> {
        self.normalize_candidates(
            solution,
            candidates,
            allowed_slots,
            state,
            reasons,
            |candidate| {
                (
                    ProviderCandidateReason::Host(candidate.reason),
                    candidate.edits,
                )
            },
            |resolver, edit, allowed| resolver.resolve_raw_index(edit, allowed),
        )
    }

    pub(super) fn resolve_static_group_and_normalize_with_state(
        &self,
        solution: &S,
        candidates: Vec<ScalarCandidate<S>>,
        allowed_slots: &[RuntimeScalarSlotId],
        state: &mut ProviderNormalizationState,
        reasons: &mut ProviderReasonArena,
    ) -> Result<Vec<ResolvedProviderCandidate<S>>, ProviderResolutionError> {
        self.normalize_candidates(
            solution,
            candidates,
            allowed_slots,
            state,
            reasons,
            |candidate| {
                (
                    ProviderCandidateReason::Static(candidate.reason()),
                    candidate.into_edits(),
                )
            },
            |resolver, edit, allowed| resolver.resolve_static_index(edit, allowed),
        )
    }

    pub(super) fn resolve_static_repair_and_normalize_with_state(
        &self,
        solution: &S,
        candidates: Vec<RepairCandidate<S>>,
        allowed_slots: &[RuntimeScalarSlotId],
        state: &mut ProviderNormalizationState,
        reasons: &mut ProviderReasonArena,
    ) -> Result<Vec<ResolvedProviderCandidate<S>>, ProviderResolutionError> {
        self.normalize_candidates(
            solution,
            candidates,
            allowed_slots,
            state,
            reasons,
            |candidate| {
                (
                    ProviderCandidateReason::Static(candidate.reason()),
                    candidate.into_edits(),
                )
            },
            |resolver, edit, allowed| resolver.resolve_static_index(edit, allowed),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn normalize_candidates<C, E, Split, Resolve>(
        &self,
        solution: &S,
        candidates: Vec<C>,
        allowed_slots: &[RuntimeScalarSlotId],
        state: &mut ProviderNormalizationState,
        reasons: &mut ProviderReasonArena,
        mut split: Split,
        mut resolve: Resolve,
    ) -> Result<Vec<ResolvedProviderCandidate<S>>, ProviderResolutionError>
    where
        E: ProviderEditAccess,
        Split: FnMut(C) -> (ProviderCandidateReason, Vec<E>),
        Resolve: FnMut(
            &RuntimeProviderSlotResolver<S>,
            &E,
            &[RuntimeScalarSlotId],
        ) -> Result<usize, ProviderResolutionError>,
    {
        let mut normalized = Vec::new();
        for candidate in candidates {
            let (reason, candidate_edits) = split(candidate);
            if candidate_edits.is_empty() {
                continue;
            }
            let mut edits = Vec::with_capacity(candidate_edits.len());
            let mut seen_targets = HashSet::new();
            let mut duplicate_target = false;
            for edit in candidate_edits {
                let slot_index = resolve(self, &edit, allowed_slots)?;
                let slot = &self.slots[slot_index];
                // Resolve every edit to its frozen numeric target before
                // duplicate testing. Raw host aliases can therefore never
                // bypass the same-target rule.
                if !seen_targets.insert((
                    slot.id.descriptor_index,
                    slot.id.variable_index,
                    edit.entity_index(),
                )) {
                    duplicate_target = true;
                    break;
                }
                let entity_index = edit.entity_index();
                let to_value = edit.to_value();
                if entity_index >= slot.slot.entity_count(solution) {
                    return Err(ProviderResolutionError::EntityIndexOutOfBounds {
                        entity_class: Arc::clone(&slot.id.entity_class),
                        variable_name: Arc::clone(&slot.id.variable_name),
                        entity_index,
                    });
                }
                if !slot.slot.value_is_legal(solution, entity_index, to_value) {
                    return Err(ProviderResolutionError::IllegalValue {
                        entity_class: Arc::clone(&slot.id.entity_class),
                        variable_name: Arc::clone(&slot.id.variable_name),
                        entity_index,
                        to_value,
                    });
                }
                edits.push(ResolvedProviderEdit {
                    descriptor_index: slot.id.descriptor_index,
                    variable_index: slot.id.variable_index,
                    slot: slot.slot.clone(),
                    entity_index,
                    to_value,
                });
            }
            if duplicate_target {
                continue;
            }
            let dedup_edits = edits
                .iter()
                .map(|edit| {
                    (
                        edit.descriptor_index,
                        edit.variable_index,
                        edit.entity_index,
                        edit.to_value,
                    )
                })
                .collect::<Vec<_>>();
            let reason = reasons.intern_candidate(reason);
            if !state.seen_candidates.insert((reason, dedup_edits)) {
                continue;
            }
            normalized.push(ResolvedProviderCandidate { reason, edits });
        }
        Ok(normalized)
    }
}
