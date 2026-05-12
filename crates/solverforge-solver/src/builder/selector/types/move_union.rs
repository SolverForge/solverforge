use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::{ListMoveUnion, Move, MoveTabuSignature, ScalarMoveUnion};

pub enum NeighborhoodMove<S, V> {
    Scalar(ScalarMoveUnion<S, usize>),
    List(ListMoveUnion<S, V>),
}

pub enum NeighborhoodMoveUndo<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    Scalar(<ScalarMoveUnion<S, usize> as Move<S>>::Undo),
    List(<ListMoveUnion<S, V> as Move<S>>::Undo),
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
        }
    }
}

impl<S, V> Move<S> for NeighborhoodMove<S, V>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Undo = NeighborhoodMoveUndo<S, V>;

    fn is_doable<D: solverforge_scoring::Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Scalar(m) => m.is_doable(score_director),
            Self::List(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: solverforge_scoring::Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        match self {
            Self::Scalar(m) => NeighborhoodMoveUndo::Scalar(m.do_move(score_director)),
            Self::List(m) => NeighborhoodMoveUndo::List(m.do_move(score_director)),
        }
    }

    fn undo_move<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &mut D,
        undo: Self::Undo,
    ) {
        match (self, undo) {
            (Self::Scalar(m), NeighborhoodMoveUndo::Scalar(undo)) => {
                m.undo_move(score_director, undo)
            }
            (Self::List(m), NeighborhoodMoveUndo::List(undo)) => m.undo_move(score_director, undo),
            _ => panic!("neighborhood move undo shape must match move shape"),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Scalar(m) => m.descriptor_index(),
            Self::List(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Scalar(m) => m.entity_indices(),
            Self::List(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Scalar(m) => m.variable_name(),
            Self::List(m) => m.variable_name(),
        }
    }

    fn requires_hard_improvement(&self) -> bool {
        match self {
            Self::Scalar(m) => m.requires_hard_improvement(),
            Self::List(m) => m.requires_hard_improvement(),
        }
    }

    fn tabu_signature<D: solverforge_scoring::Director<S>>(
        &self,
        score_director: &D,
    ) -> MoveTabuSignature {
        match self {
            Self::Scalar(m) => m.tabu_signature(score_director),
            Self::List(m) => m.tabu_signature(score_director),
        }
    }

    fn for_each_affected_entity(
        &self,
        visitor: &mut dyn FnMut(crate::heuristic::r#move::MoveAffectedEntity<'_>),
    ) {
        match self {
            Self::Scalar(m) => m.for_each_affected_entity(visitor),
            Self::List(m) => m.for_each_affected_entity(visitor),
        }
    }
}
