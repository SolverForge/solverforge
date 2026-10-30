//! ListMoveUnion - a monomorphized union of all list-variable move types.
//!
//! This allows local search to combine all list move types in a single arena
//! without trait-object dispatch.

use std::borrow::Cow;
use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::stats::CandidateTraceIdentity;

use super::{
    ListChangeMove, ListMultiSwapMove, ListPermuteMove, ListReverseMove, ListRuinMove,
    ListSwapMove, Move, MoveTabuSignature, SublistChangeMove, SublistSwapMove,
};

/// One move carrier for every move the reference precedence selector emits,
/// without type erasure. Test-only: it backs the parity oracles.
#[allow(clippy::large_enum_variant)]
pub(crate) enum ListMoveUnion<S, V> {
    ListChange(ListChangeMove<S, V>),
    ListSwap(ListSwapMove<S, V>),
    ListMultiSwap(ListMultiSwapMove<S, V>),
    ListPermute(ListPermuteMove<S, V>),
    SublistChange(SublistChangeMove<S, V>),
    SublistSwap(SublistSwapMove<S, V>),
    ListReverse(ListReverseMove<S, V>),
    ListRuin(ListRuinMove<S, V>),
}

pub(crate) enum ListMoveUnionUndo<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    ListChange(<ListChangeMove<S, V> as Move<S>>::Undo),
    ListSwap(<ListSwapMove<S, V> as Move<S>>::Undo),
    ListMultiSwap(<ListMultiSwapMove<S, V> as Move<S>>::Undo),
    ListPermute(<ListPermuteMove<S, V> as Move<S>>::Undo),
    SublistChange(<SublistChangeMove<S, V> as Move<S>>::Undo),
    SublistSwap(<SublistSwapMove<S, V> as Move<S>>::Undo),
    ListReverse(<ListReverseMove<S, V> as Move<S>>::Undo),
    ListRuin(<ListRuinMove<S, V> as Move<S>>::Undo),
}

impl<S, V> Debug for ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ListChange(m) => m.fmt(f),
            Self::ListSwap(m) => m.fmt(f),
            Self::ListMultiSwap(m) => m.fmt(f),
            Self::ListPermute(m) => m.fmt(f),
            Self::SublistChange(m) => m.fmt(f),
            Self::SublistSwap(m) => m.fmt(f),
            Self::ListReverse(m) => m.fmt(f),
            Self::ListRuin(m) => m.fmt(f),
        }
    }
}

impl<S, V> Move<S> for ListMoveUnion<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Undo = ListMoveUnionUndo<S, V>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::ListChange(m) => m.is_doable(score_director),
            Self::ListSwap(m) => m.is_doable(score_director),
            Self::ListMultiSwap(m) => m.is_doable(score_director),
            Self::ListPermute(m) => m.is_doable(score_director),
            Self::SublistChange(m) => m.is_doable(score_director),
            Self::SublistSwap(m) => m.is_doable(score_director),
            Self::ListReverse(m) => m.is_doable(score_director),
            Self::ListRuin(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        match self {
            Self::ListChange(m) => {
                m.do_move(score_director);
                ListMoveUnionUndo::ListChange(())
            }
            Self::ListSwap(m) => {
                m.do_move(score_director);
                ListMoveUnionUndo::ListSwap(())
            }
            Self::ListMultiSwap(m) => {
                m.do_move(score_director);
                ListMoveUnionUndo::ListMultiSwap(())
            }
            Self::ListPermute(m) => ListMoveUnionUndo::ListPermute(m.do_move(score_director)),
            Self::SublistChange(m) => {
                m.do_move(score_director);
                ListMoveUnionUndo::SublistChange(())
            }
            Self::SublistSwap(m) => {
                m.do_move(score_director);
                ListMoveUnionUndo::SublistSwap(())
            }
            Self::ListReverse(m) => {
                m.do_move(score_director);
                ListMoveUnionUndo::ListReverse(())
            }
            Self::ListRuin(m) => ListMoveUnionUndo::ListRuin(m.do_move(score_director)),
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        match (self, undo) {
            (Self::ListChange(m), ListMoveUnionUndo::ListChange(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::ListSwap(m), ListMoveUnionUndo::ListSwap(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::ListMultiSwap(m), ListMoveUnionUndo::ListMultiSwap(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::ListPermute(m), ListMoveUnionUndo::ListPermute(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::SublistChange(m), ListMoveUnionUndo::SublistChange(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::SublistSwap(m), ListMoveUnionUndo::SublistSwap(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::ListReverse(m), ListMoveUnionUndo::ListReverse(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::ListRuin(m), ListMoveUnionUndo::ListRuin(undo)) => {
                m.undo_move(score_director, undo)
            }
            _ => panic!("list move undo shape must match move shape"),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::ListChange(m) => m.descriptor_index(),
            Self::ListSwap(m) => m.descriptor_index(),
            Self::ListMultiSwap(m) => m.descriptor_index(),
            Self::ListPermute(m) => m.descriptor_index(),
            Self::SublistChange(m) => m.descriptor_index(),
            Self::SublistSwap(m) => m.descriptor_index(),
            Self::ListReverse(m) => m.descriptor_index(),
            Self::ListRuin(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::ListChange(m) => m.entity_indices(),
            Self::ListSwap(m) => m.entity_indices(),
            Self::ListMultiSwap(m) => m.entity_indices(),
            Self::ListPermute(m) => m.entity_indices(),
            Self::SublistChange(m) => m.entity_indices(),
            Self::SublistSwap(m) => m.entity_indices(),
            Self::ListReverse(m) => m.entity_indices(),
            Self::ListRuin(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::ListChange(m) => m.variable_name(),
            Self::ListSwap(m) => m.variable_name(),
            Self::ListMultiSwap(m) => m.variable_name(),
            Self::ListPermute(m) => m.variable_name(),
            Self::SublistChange(m) => m.variable_name(),
            Self::SublistSwap(m) => m.variable_name(),
            Self::ListReverse(m) => m.variable_name(),
            Self::ListRuin(m) => m.variable_name(),
        }
    }

    fn telemetry_label(&self) -> &'static str {
        match self {
            Self::ListChange(m) => m.telemetry_label(),
            Self::ListSwap(m) => m.telemetry_label(),
            Self::ListMultiSwap(m) => m.telemetry_label(),
            Self::ListPermute(m) => m.telemetry_label(),
            Self::SublistChange(m) => m.telemetry_label(),
            Self::SublistSwap(m) => m.telemetry_label(),
            Self::ListReverse(m) => m.telemetry_label(),
            Self::ListRuin(m) => m.telemetry_label(),
        }
    }

    fn requires_hard_improvement(&self) -> bool {
        match self {
            Self::ListChange(m) => m.requires_hard_improvement(),
            Self::ListSwap(m) => m.requires_hard_improvement(),
            Self::ListMultiSwap(m) => m.requires_hard_improvement(),
            Self::ListPermute(m) => m.requires_hard_improvement(),
            Self::SublistChange(m) => m.requires_hard_improvement(),
            Self::SublistSwap(m) => m.requires_hard_improvement(),
            Self::ListReverse(m) => m.requires_hard_improvement(),
            Self::ListRuin(m) => m.requires_hard_improvement(),
        }
    }

    fn requires_score_improvement(&self) -> bool {
        match self {
            Self::ListChange(m) => m.requires_score_improvement(),
            Self::ListSwap(m) => m.requires_score_improvement(),
            Self::ListMultiSwap(m) => m.requires_score_improvement(),
            Self::ListPermute(m) => m.requires_score_improvement(),
            Self::SublistChange(m) => m.requires_score_improvement(),
            Self::SublistSwap(m) => m.requires_score_improvement(),
            Self::ListReverse(m) => m.requires_score_improvement(),
            Self::ListRuin(m) => m.requires_score_improvement(),
        }
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> Cow<'_, MoveTabuSignature> {
        match self {
            Self::ListChange(m) => m.tabu_signature(score_director),
            Self::ListSwap(m) => m.tabu_signature(score_director),
            Self::ListMultiSwap(m) => m.tabu_signature(score_director),
            Self::ListPermute(m) => m.tabu_signature(score_director),
            Self::SublistChange(m) => m.tabu_signature(score_director),
            Self::SublistSwap(m) => m.tabu_signature(score_director),
            Self::ListReverse(m) => m.tabu_signature(score_director),
            Self::ListRuin(m) => m.tabu_signature(score_director),
        }
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        match self {
            Self::ListChange(m) => m.candidate_trace_identity(),
            Self::ListSwap(m) => m.candidate_trace_identity(),
            Self::ListMultiSwap(m) => m.candidate_trace_identity(),
            Self::ListPermute(m) => m.candidate_trace_identity(),
            Self::SublistChange(m) => m.candidate_trace_identity(),
            Self::SublistSwap(m) => m.candidate_trace_identity(),
            Self::ListReverse(m) => m.candidate_trace_identity(),
            Self::ListRuin(m) => m.candidate_trace_identity(),
        }
    }
}
