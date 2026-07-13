//! One owned move carrier for compiled local-search leaves.

use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::selector::types::SequentialMoveCarrier;
use crate::heuristic::r#move::{
    Move, MoveAffectedEntity, MoveTabuSignature, RuntimeCompoundMove, ScalarMoveUnion,
    SequentialCompositeMove,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::scalar_neighborhood::RuntimeScalarMove;
use crate::stats::CandidateTraceIdentity;

use super::super::RuntimeListMove;

/// Fully concrete carrier for scalar, list, assignment-group, provider, and
/// selected Cartesian moves. It is the only carrier produced by compiled
/// local-search lowering; no second selector union is rebuilt beside it.
#[allow(clippy::large_enum_variant)]
pub(crate) enum RuntimeNeighborhoodMove<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    Scalar(RuntimeScalarMove<S>),
    List(RuntimeListMove<S, V, DM, IDM>),
    Grouped(ScalarMoveUnion<S, usize>),
    Provider(RuntimeCompoundMove<S>),
    Sequential(SequentialCompositeMove<S, Self>),
}

pub(crate) enum RuntimeNeighborhoodMoveUndo<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    Scalar(<RuntimeScalarMove<S> as Move<S>>::Undo),
    List(<RuntimeListMove<S, V, DM, IDM> as Move<S>>::Undo),
    Grouped(<ScalarMoveUnion<S, usize> as Move<S>>::Undo),
    Provider(<RuntimeCompoundMove<S> as Move<S>>::Undo),
    Sequential(Box<(Self, Self)>),
}

impl<S, V, DM, IDM> Debug for RuntimeNeighborhoodMove<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(mov) => formatter
                .debug_tuple("RuntimeNeighborhoodMove::Scalar")
                .field(mov)
                .finish(),
            Self::List(mov) => formatter
                .debug_tuple("RuntimeNeighborhoodMove::List")
                .field(mov)
                .finish(),
            Self::Grouped(mov) => formatter
                .debug_tuple("RuntimeNeighborhoodMove::Grouped")
                .field(mov)
                .finish(),
            Self::Provider(mov) => formatter
                .debug_tuple("RuntimeNeighborhoodMove::Provider")
                .field(mov)
                .finish(),
            Self::Sequential(mov) => formatter
                .debug_tuple("RuntimeNeighborhoodMove::Sequential")
                .field(mov)
                .finish(),
        }
    }
}

impl<S, V, DM, IDM> Move<S> for RuntimeNeighborhoodMove<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    type Undo = RuntimeNeighborhoodMoveUndo<S, V, DM, IDM>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Scalar(mov) => mov.is_doable(score_director),
            Self::List(mov) => mov.is_doable(score_director),
            Self::Grouped(mov) => mov.is_doable(score_director),
            Self::Provider(mov) => mov.is_doable(score_director),
            Self::Sequential(mov) => mov.is_doable(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        match self {
            Self::Scalar(mov) => RuntimeNeighborhoodMoveUndo::Scalar(mov.do_move(score_director)),
            Self::List(mov) => RuntimeNeighborhoodMoveUndo::List(mov.do_move(score_director)),
            Self::Grouped(mov) => RuntimeNeighborhoodMoveUndo::Grouped(mov.do_move(score_director)),
            Self::Provider(mov) => {
                RuntimeNeighborhoodMoveUndo::Provider(mov.do_move(score_director))
            }
            Self::Sequential(mov) => {
                RuntimeNeighborhoodMoveUndo::Sequential(Box::new(mov.do_move(score_director)))
            }
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        match (self, undo) {
            (Self::Scalar(mov), RuntimeNeighborhoodMoveUndo::Scalar(undo)) => {
                mov.undo_move(score_director, undo)
            }
            (Self::List(mov), RuntimeNeighborhoodMoveUndo::List(undo)) => {
                mov.undo_move(score_director, undo)
            }
            (Self::Grouped(mov), RuntimeNeighborhoodMoveUndo::Grouped(undo)) => {
                mov.undo_move(score_director, undo)
            }
            (Self::Provider(mov), RuntimeNeighborhoodMoveUndo::Provider(undo)) => {
                mov.undo_move(score_director, undo)
            }
            (Self::Sequential(mov), RuntimeNeighborhoodMoveUndo::Sequential(undo)) => {
                mov.undo_move(score_director, *undo)
            }
            _ => panic!("compiled runtime neighborhood move undo shape must match move kind"),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Scalar(mov) => mov.descriptor_index(),
            Self::List(mov) => mov.descriptor_index(),
            Self::Grouped(mov) => mov.descriptor_index(),
            Self::Provider(mov) => mov.descriptor_index(),
            Self::Sequential(mov) => mov.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Scalar(mov) => mov.entity_indices(),
            Self::List(mov) => mov.entity_indices(),
            Self::Grouped(mov) => mov.entity_indices(),
            Self::Provider(mov) => mov.entity_indices(),
            Self::Sequential(mov) => mov.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Scalar(mov) => mov.variable_name(),
            Self::List(mov) => mov.variable_name(),
            Self::Grouped(mov) => mov.variable_name(),
            Self::Provider(mov) => mov.variable_name(),
            Self::Sequential(mov) => mov.variable_name(),
        }
    }

    fn telemetry_label(&self) -> &'static str {
        match self {
            Self::Scalar(mov) => mov.telemetry_label(),
            Self::List(mov) => mov.telemetry_label(),
            Self::Grouped(mov) => mov.telemetry_label(),
            Self::Provider(mov) => mov.telemetry_label(),
            Self::Sequential(mov) => mov.telemetry_label(),
        }
    }

    fn requires_hard_improvement(&self) -> bool {
        match self {
            Self::Scalar(mov) => mov.requires_hard_improvement(),
            Self::List(mov) => mov.requires_hard_improvement(),
            Self::Grouped(mov) => mov.requires_hard_improvement(),
            Self::Provider(mov) => mov.requires_hard_improvement(),
            Self::Sequential(mov) => mov.requires_hard_improvement(),
        }
    }

    fn requires_score_improvement(&self) -> bool {
        match self {
            Self::Scalar(mov) => mov.requires_score_improvement(),
            Self::List(mov) => mov.requires_score_improvement(),
            Self::Grouped(mov) => mov.requires_score_improvement(),
            Self::Provider(mov) => mov.requires_score_improvement(),
            Self::Sequential(mov) => mov.requires_score_improvement(),
        }
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        match self {
            Self::Scalar(mov) => mov.tabu_signature(score_director),
            Self::List(mov) => mov.tabu_signature(score_director),
            Self::Grouped(mov) => mov.tabu_signature(score_director),
            Self::Provider(mov) => mov.tabu_signature(score_director),
            Self::Sequential(mov) => mov.tabu_signature(score_director),
        }
    }

    fn candidate_trace_identity(&self) -> Option<CandidateTraceIdentity> {
        match self {
            Self::Scalar(mov) => mov.candidate_trace_identity(),
            Self::List(mov) => mov.candidate_trace_identity(),
            Self::Grouped(mov) => mov.candidate_trace_identity(),
            Self::Provider(mov) => mov.candidate_trace_identity(),
            Self::Sequential(mov) => mov.candidate_trace_identity(),
        }
    }

    fn for_each_affected_entity(&self, visitor: &mut dyn FnMut(MoveAffectedEntity<'_>)) {
        match self {
            Self::Scalar(mov) => mov.for_each_affected_entity(visitor),
            Self::List(mov) => mov.for_each_affected_entity(visitor),
            Self::Grouped(mov) => mov.for_each_affected_entity(visitor),
            Self::Provider(mov) => mov.for_each_affected_entity(visitor),
            Self::Sequential(mov) => mov.for_each_affected_entity(visitor),
        }
    }
}

impl<S, V, DM, IDM> SequentialMoveCarrier<S> for RuntimeNeighborhoodMove<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S>,
{
    fn from_sequential(composite: SequentialCompositeMove<S, Self>) -> Self {
        Self::Sequential(composite)
    }
}
