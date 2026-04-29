use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::{
    ListMoveUnion, Move, MoveTabuSignature, ScalarMoveUnion, SequentialCompositeMove,
};

pub enum NeighborhoodMove<S, V> {
    Scalar(ScalarMoveUnion<S, usize>),
    List(ListMoveUnion<S, V>),
    Composite(SequentialCompositeMove<S, NeighborhoodMove<S, V>>),
}

impl<S, V> Clone for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn clone(&self) -> Self {
        match self {
            Self::Scalar(m) => Self::Scalar(m.clone()),
            Self::List(m) => Self::List(m.clone()),
            Self::Composite(m) => Self::Composite(m.clone()),
        }
    }
}

impl<S, V> Debug for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(m) => write!(f, "NeighborhoodMove::Scalar({m:?})"),
            Self::List(m) => write!(f, "NeighborhoodMove::List({m:?})"),
            Self::Composite(m) => write!(f, "NeighborhoodMove::Composite({m:?})"),
        }
    }
}

impl<S, V> Move<S> for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Scalar(m) => m.is_doable(score_director),
            Self::List(m) => m.is_doable(score_director),
            Self::Composite(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: solverforge_scoring::Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Scalar(m) => m.do_move(score_director),
            Self::List(m) => m.do_move(score_director),
            Self::Composite(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Scalar(m) => m.descriptor_index(),
            Self::List(m) => m.descriptor_index(),
            Self::Composite(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Scalar(m) => m.entity_indices(),
            Self::List(m) => m.entity_indices(),
            Self::Composite(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Scalar(m) => m.variable_name(),
            Self::List(m) => m.variable_name(),
            Self::Composite(m) => m.variable_name(),
        }
    }

    fn requires_hard_improvement(&self) -> bool {
        match self {
            Self::Scalar(m) => m.requires_hard_improvement(),
            Self::List(m) => m.requires_hard_improvement(),
            Self::Composite(m) => m.requires_hard_improvement(),
        }
    }

    fn tabu_signature<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
    ) -> MoveTabuSignature {
        match self {
            Self::Scalar(m) => m.tabu_signature(score_director),
            Self::List(m) => m.tabu_signature(score_director),
            Self::Composite(m) => m.tabu_signature(score_director),
        }
    }

    fn for_each_affected_entity(
        &self,
        visitor: &mut dyn FnMut(crate::heuristic::r#move::MoveAffectedEntity<'_>),
    ) {
        match self {
            Self::Scalar(m) => m.for_each_affected_entity(visitor),
            Self::List(m) => m.for_each_affected_entity(visitor),
            Self::Composite(m) => m.for_each_affected_entity(visitor),
        }
    }
}
