use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::compound_scalar::{CompoundScalarEdit, CompoundScalarMove};
use super::{Move, MoveAffectedEntity, MoveTabuSignature};

const CONFLICT_REPAIR_VARIABLE: &str = "conflict_repair";

#[derive(Clone)]
pub struct ConflictRepairScalarEdit<S> {
    pub descriptor_index: usize,
    pub entity_index: usize,
    pub variable_index: usize,
    pub variable_name: &'static str,
    pub to_value: Option<usize>,
    pub getter: fn(&S, usize, usize) -> Option<usize>,
    pub setter: fn(&mut S, usize, usize, Option<usize>),
}

impl<S> Debug for ConflictRepairScalarEdit<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConflictRepairScalarEdit")
            .field("descriptor_index", &self.descriptor_index)
            .field("entity_index", &self.entity_index)
            .field("variable_index", &self.variable_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

#[derive(Clone)]
pub struct ConflictRepairMove<S> {
    compound: CompoundScalarMove<S>,
}

impl<S> ConflictRepairMove<S> {
    pub fn new(reason: &'static str, edits: Vec<ConflictRepairScalarEdit<S>>) -> Self {
        let edits = edits
            .into_iter()
            .map(|edit| CompoundScalarEdit {
                descriptor_index: edit.descriptor_index,
                entity_index: edit.entity_index,
                variable_index: edit.variable_index,
                variable_name: edit.variable_name,
                to_value: edit.to_value,
                getter: edit.getter,
                setter: edit.setter,
                value_is_legal: None,
            })
            .collect();
        Self {
            compound: CompoundScalarMove::with_label(reason, CONFLICT_REPAIR_VARIABLE, edits),
        }
    }
}

impl<S> Debug for ConflictRepairMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ConflictRepairMove")
            .field(&self.compound)
            .finish()
    }
}

impl<S> Move<S> for ConflictRepairMove<S>
where
    S: PlanningSolution,
{
    type Undo = <CompoundScalarMove<S> as Move<S>>::Undo;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        self.compound.is_doable(score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        self.compound.do_move(score_director)
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        self.compound.undo_move(score_director, undo);
    }

    fn descriptor_index(&self) -> usize {
        self.compound.descriptor_index()
    }

    fn entity_indices(&self) -> &[usize] {
        self.compound.entity_indices()
    }

    fn variable_name(&self) -> &str {
        self.compound.variable_name()
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        self.compound.tabu_signature(score_director)
    }

    fn for_each_affected_entity(&self, visitor: &mut dyn FnMut(MoveAffectedEntity<'_>)) {
        self.compound.for_each_affected_entity(visitor);
    }
}
