use std::fmt;

use smallvec::smallvec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::list_access::ListAccess;
use crate::builder::context::{RuntimeListElement, RuntimeListSlot, RuntimeScalarSlot};
use crate::heuristic::r#move::metadata::{
    encode_option_debug, encode_usize, hash_str, MoveTabuScope,
};
use crate::heuristic::r#move::{Move, MoveTabuSignature};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::stats::{CandidateTraceCoordinate, CandidateTraceIdentity};

/// One scalar candidate over the common typed/dynamic runtime slot carrier.
/// The physical dispatch is limited to the carrier legality distinction: typed
/// candidate providers are authoritative, while dynamic slots retain their
/// backend legality check.
pub(super) struct RuntimeScalarConstructionMove<S> {
    slot: RuntimeScalarSlot<S>,
    entity_index: usize,
    value: usize,
}

impl<S> RuntimeScalarConstructionMove<S> {
    pub(super) fn new(slot: RuntimeScalarSlot<S>, entity_index: usize, value: usize) -> Self {
        Self {
            slot,
            entity_index,
            value,
        }
    }

    pub(super) fn slot(&self) -> &RuntimeScalarSlot<S> {
        &self.slot
    }

    pub(super) fn entity_index(&self) -> usize {
        self.entity_index
    }

    pub(super) fn value(&self) -> usize {
        self.value
    }
}

impl<S> fmt::Debug for RuntimeScalarConstructionMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeScalarConstructionMove")
            .field("slot", &self.slot)
            .field("entity_index", &self.entity_index)
            .field("value", &self.value)
            .finish()
    }
}

impl<S> Move<S> for RuntimeScalarConstructionMove<S>
where
    S: PlanningSolution,
{
    type Undo = Option<usize>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        if self.entity_index >= self.slot.entity_count(solution)
            || self.slot.current_value(solution, self.entity_index) == Some(self.value)
        {
            return false;
        }
        match &self.slot {
            RuntimeScalarSlot::Static(_) => true,
            RuntimeScalarSlot::Dynamic(_) => {
                self.slot
                    .value_is_legal(solution, self.entity_index, Some(self.value))
            }
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let previous = self
            .slot
            .current_value(score_director.working_solution(), self.entity_index);
        score_director.before_variable_changed(self.slot.descriptor_index(), self.entity_index);
        self.slot.set_value(
            score_director.working_solution_mut(),
            self.entity_index,
            Some(self.value),
        );
        score_director.after_variable_changed(self.slot.descriptor_index(), self.entity_index);
        previous
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        score_director.before_variable_changed(self.slot.descriptor_index(), self.entity_index);
        self.slot.set_value(
            score_director.working_solution_mut(),
            self.entity_index,
            undo,
        );
        score_director.after_variable_changed(self.slot.descriptor_index(), self.entity_index);
    }

    fn descriptor_index(&self) -> usize {
        self.slot.descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.slot.variable_name()
    }

    fn telemetry_label(&self) -> &'static str {
        match &self.slot {
            RuntimeScalarSlot::Static(_) => "move",
            RuntimeScalarSlot::Dynamic(_) => "dynamic_scalar_change",
        }
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let current = self
            .slot
            .current_value(score_director.working_solution(), self.entity_index);
        let from = encode_option_debug(current.as_ref());
        let to = encode_option_debug(Some(self.value).as_ref());
        let entity = encode_usize(self.entity_index);
        let scope = MoveTabuScope::new(self.slot.descriptor_index(), self.slot.variable_name());
        let variable = hash_str(self.slot.variable_name());
        MoveTabuSignature::new(
            scope,
            smallvec![
                encode_usize(self.slot.descriptor_index()),
                variable,
                entity,
                from,
                to
            ],
            smallvec![
                encode_usize(self.slot.descriptor_index()),
                variable,
                entity,
                to,
                from
            ],
        )
        .with_entity_tokens([scope.entity_token(entity)])
        .with_destination_value_tokens([scope.value_token(to)])
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        match &self.slot {
            // Preserve the established descriptor/static trace contract: it
            // has no cross-representation identity rather than inventing one
            // during this refactor.
            RuntimeScalarSlot::Static(_) => None,
            RuntimeScalarSlot::Dynamic(_) => Some(CandidateTraceIdentity::logical_move(
                self.slot.descriptor_index(),
                self.slot.variable_name(),
                "scalar_change",
                [
                    CandidateTraceCoordinate::from(self.entity_index),
                    CandidateTraceCoordinate::from(Some(self.value)),
                ],
            )),
        }
    }
}

/// One insertion of a frozen declared list element. It is intentionally a
/// construction move, not a generic list-change move: the source coordinate
/// is the immutable declared-stream index rather than a transient list
/// position.
pub(super) struct RuntimeListInsertionMove<S, V, DM, IDM> {
    slot: RuntimeListSlot<S, V, DM, IDM>,
    element: RuntimeListElement<V>,
    source_index: usize,
    entity_index: usize,
    position: usize,
}

impl<S, V, DM, IDM> RuntimeListInsertionMove<S, V, DM, IDM> {
    pub(super) fn new(
        slot: RuntimeListSlot<S, V, DM, IDM>,
        element: RuntimeListElement<V>,
        source_index: usize,
        entity_index: usize,
        position: usize,
    ) -> Self {
        Self {
            slot,
            element,
            source_index,
            entity_index,
            position,
        }
    }
}

impl<S, V, DM, IDM> fmt::Debug for RuntimeListInsertionMove<S, V, DM, IDM>
where
    RuntimeListSlot<S, V, DM, IDM>: fmt::Debug,
    RuntimeListElement<V>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeListInsertionMove")
            .field("slot", &self.slot)
            .field("source_index", &self.source_index)
            .field("element", &self.element)
            .field("entity_index", &self.entity_index)
            .field("position", &self.position)
            .finish()
    }
}

impl<S, V, DM, IDM> Move<S> for RuntimeListInsertionMove<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Undo = RuntimeListElement<V>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();
        if self.entity_index >= self.slot.entity_count(solution)
            || self.position > self.slot.list_len(solution, self.entity_index)
        {
            return false;
        }
        match self.slot.element_owner(solution, &self.element) {
            Ok(None) => true,
            Ok(Some(owner)) => owner == self.entity_index,
            Err(_) => false,
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        score_director.before_variable_changed(self.slot.descriptor_index(), self.entity_index);
        self.slot.list_insert(
            score_director.working_solution_mut(),
            self.entity_index,
            self.position,
            self.element.clone(),
        );
        score_director.after_variable_changed(self.slot.descriptor_index(), self.entity_index);
        self.element.clone()
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        score_director.before_variable_changed(self.slot.descriptor_index(), self.entity_index);
        let removed = self.slot.list_remove(
            score_director.working_solution_mut(),
            self.entity_index,
            self.position,
        );
        debug_assert_eq!(removed, Some(undo));
        score_director.after_variable_changed(self.slot.descriptor_index(), self.entity_index);
    }

    fn descriptor_index(&self) -> usize {
        self.slot.descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.slot.variable_name()
    }

    fn telemetry_label(&self) -> &'static str {
        "runtime_list_insertion"
    }

    fn tabu_signature<D: Director<S>>(&self, _score_director: &D) -> MoveTabuSignature {
        let scope = MoveTabuScope::new(self.slot.descriptor_index(), self.slot.variable_name());
        let source = encode_usize(self.source_index);
        let entity = encode_usize(self.entity_index);
        let position = encode_usize(self.position);
        let variable = hash_str(self.slot.variable_name());
        MoveTabuSignature::new(
            scope,
            smallvec![
                encode_usize(self.slot.descriptor_index()),
                variable,
                source,
                entity,
                position
            ],
            smallvec![
                encode_usize(self.slot.descriptor_index()),
                variable,
                source,
                entity,
                position
            ],
        )
        .with_entity_tokens([scope.entity_token(entity)])
        .with_destination_value_tokens([scope.value_token(source)])
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        Some(CandidateTraceIdentity::logical_move(
            self.slot.descriptor_index(),
            self.slot.variable_name(),
            "list_insertion",
            [self.source_index, self.entity_index, self.position],
        ))
    }
}
