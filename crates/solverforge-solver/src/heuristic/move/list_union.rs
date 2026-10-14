/* ListMoveUnion - a monomorphized union of all list-variable move types.

This allows local search to combine all list move types in a single arena
without trait-object dispatch.
*/

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::stats::CandidateTraceIdentity;

use super::{
    KOptMove, ListChangeMove, ListMultiSwapMove, ListPermuteMove, ListReverseMove, ListRuinMove,
    ListSwapMove, Move, MoveTabuSignature, SublistChangeMove, SublistSwapMove,
};

/// A monomorphized union of all list-variable move types.
///
/// Implements `Move<S>` by delegating to the inner variant.
/// Enables combining `ListChangeMoveSelector`, `ListSwapMoveSelector`,
/// `ListPermuteMoveSelector`, `SublistChangeMoveSelector`, `SublistSwapMoveSelector`,
/// `ListReverseMoveSelector`, `KOptMoveSelector`, and
/// `ListRuinMoveSelector` without type erasure.
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::ListMoveUnion;
/// ```
#[allow(clippy::large_enum_variant)]
pub enum ListMoveUnion<S, V> {
    ListChange(ListChangeMove<S, V>),
    ListSwap(ListSwapMove<S, V>),
    ListMultiSwap(ListMultiSwapMove<S, V>),
    ListPermute(ListPermuteMove<S, V>),
    SublistChange(SublistChangeMove<S, V>),
    SublistSwap(SublistSwapMove<S, V>),
    ListReverse(ListReverseMove<S, V>),
    KOpt(KOptMove<S, V>),
    ListRuin(ListRuinMove<S, V>),
}

pub enum ListMoveUnionUndo<S, V>
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
    KOpt(<KOptMove<S, V> as Move<S>>::Undo),
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
            Self::KOpt(m) => m.fmt(f),
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
            Self::KOpt(m) => m.is_doable(score_director),
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
            Self::KOpt(m) => ListMoveUnionUndo::KOpt(m.do_move(score_director)),
            Self::ListRuin(m) => ListMoveUnionUndo::ListRuin(m.do_move(score_director)),
        }
    }

    fn do_move_with_scratch<D: Director<S>>(
        &self,
        score_director: &mut D,
        journal_pool: &mut Vec<Self::Undo>,
    ) -> Self::Undo {
        match self {
            Self::KOpt(m) => {
                // The outer pool stores union undos; a K-opt-shaped entry
                // carries an inner journal whose inlined capacity this arm
                // reuses.
                let mut journal = journal_pool
                    .iter()
                    .rposition(|undo| matches!(undo, ListMoveUnionUndo::KOpt(_)))
                    .map(|index| match journal_pool.swap_remove(index) {
                        ListMoveUnionUndo::KOpt(journal) => journal,
                        _ => unreachable!("rposition matched the KOpt shape"),
                    })
                    .unwrap_or_default();
                m.do_move_into_journal(score_director, &mut journal);
                ListMoveUnionUndo::KOpt(journal)
            }
            other => other.do_move(score_director),
        }
    }

    fn undo_move_with_scratch<D: Director<S>>(
        &self,
        score_director: &mut D,
        undo: Self::Undo,
        journal_pool: &mut Vec<Self::Undo>,
    ) {
        match (self, undo) {
            (Self::KOpt(m), ListMoveUnionUndo::KOpt(undo)) => {
                m.undo_move(score_director, undo);
                journal_pool.push(ListMoveUnionUndo::KOpt(
                    <KOptMove<S, V> as Move<S>>::Undo::new(),
                ));
            }
            (other, undo) => other.undo_move(score_director, undo),
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
            (Self::KOpt(m), ListMoveUnionUndo::KOpt(undo)) => m.undo_move(score_director, undo),
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
            Self::KOpt(m) => m.descriptor_index(),
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
            Self::KOpt(m) => m.entity_indices(),
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
            Self::KOpt(m) => m.variable_name(),
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
            Self::KOpt(m) => m.telemetry_label(),
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
            Self::KOpt(m) => m.requires_hard_improvement(),
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
            Self::KOpt(m) => m.requires_score_improvement(),
            Self::ListRuin(m) => m.requires_score_improvement(),
        }
    }

    fn borrowed_tabu_signature(&self) -> Option<&MoveTabuSignature> {
        match self {
            Self::ListChange(m) => m.borrowed_tabu_signature(),
            Self::ListSwap(m) => m.borrowed_tabu_signature(),
            Self::ListMultiSwap(m) => m.borrowed_tabu_signature(),
            Self::ListPermute(m) => m.borrowed_tabu_signature(),
            Self::SublistChange(m) => m.borrowed_tabu_signature(),
            Self::SublistSwap(m) => m.borrowed_tabu_signature(),
            Self::ListReverse(m) => m.borrowed_tabu_signature(),
            Self::KOpt(m) => m.borrowed_tabu_signature(),
            Self::ListRuin(m) => m.borrowed_tabu_signature(),
        }
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        match self {
            Self::ListChange(m) => m.tabu_signature(score_director),
            Self::ListSwap(m) => m.tabu_signature(score_director),
            Self::ListMultiSwap(m) => m.tabu_signature(score_director),
            Self::ListPermute(m) => m.tabu_signature(score_director),
            Self::SublistChange(m) => m.tabu_signature(score_director),
            Self::SublistSwap(m) => m.tabu_signature(score_director),
            Self::ListReverse(m) => m.tabu_signature(score_director),
            Self::KOpt(m) => m.tabu_signature(score_director),
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
            Self::KOpt(m) => m.candidate_trace_identity(),
            Self::ListRuin(m) => m.candidate_trace_identity(),
        }
    }
}
