//! One indexed selector-composition declaration arena.
//!
//! The compiled runner supplies frozen leaves. Structural edges are stable
//! coordinates; declaration and stream arenas are owned independently.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::{UnionSelectionOrder, UnionWeighting};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{Move, SequentialCompositeMove};
use crate::heuristic::selector::move_selector::{MoveStreamContext, ResourceMoveCursor};

mod cartesian;
mod execution;
mod state;

#[cfg(test)]
mod tests;

pub(crate) use execution::SelectorCompositionState;
pub(crate) use state::SelectorCompositionCursor;

/// A move carrier that can own a selected Cartesian pair.
///
/// The generic compositor never creates a second runtime-specific move union.
/// Each carrier provides its one lossless owned-sequence representation.
pub(crate) trait SequentialMoveCarrier<S>: Move<S> + Sized
where
    S: PlanningSolution,
{
    fn from_sequential(composite: SequentialCompositeMove<S, Self>) -> Self;
}

/// A frozen flat leaf set whose stream state belongs to a composition
/// execution, not to the selector definition.
///
/// The stateful opening hook returns the composition cursor directly.
///
/// This intentionally does not inherit the public selector facade. Provider
/// leaves borrow the runner-owned resource only when a reachable child is
/// pulled.
pub(crate) trait StatefulComposedFlat<S, M, FlatState, Resources>
where
    S: PlanningSolution,
    M: Move<S>,
{
    type Cursor<'a>: ResourceMoveCursor<S, M, Resources> + 'a
    where
        Self: 'a;

    fn new_stream_state(&self) -> FlatState;

    fn open_cursor_with_stream_state<'a, D: Director<S>>(
        &'a self,
        stream_state: &mut FlatState,
        resources: &mut Resources,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a>;

    fn size<D: Director<S>>(&self, score_director: &D) -> usize;

    fn validate_cursor<D: Director<S>>(&self, score_director: &D);
}

/// Stable declaration and stream-state coordinate, never a candidate ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompositionNodeId(usize);

impl CompositionNodeId {
    pub(super) fn index(self) -> usize {
        self.0
    }
}

/// Structural variants are closed; arbitrary flat extensions remain concrete.
#[derive(Debug)]
pub(crate) enum SelectorCompositionNode<Flat> {
    Flat(Flat),
    Limited {
        selector: CompositionNodeId,
        selected_count_limit: usize,
    },
    Union {
        selection_order: UnionSelectionOrder,
        weighting: UnionWeighting,
        weights: Vec<u64>,
        children: Vec<CompositionNodeId>,
    },
    Cartesian {
        left: CompositionNodeId,
        right: CompositionNodeId,
        require_hard_improvement: bool,
    },
}

/// Built once in postorder, then owned immutably by one solve execution.
/// Children precede parents, and each child has exactly one parent. Thus
/// cursor state can use the same coordinates without sharing mutable leaves.
pub(crate) struct SelectorComposition<S, M, Flat, FlatState> {
    nodes: Vec<SelectorCompositionNode<Flat>>,
    _marker: PhantomData<fn() -> (S, M, FlatState)>,
}

impl<S, M, Flat, FlatState> SelectorComposition<S, M, Flat, FlatState> {
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn push(&mut self, node: SelectorCompositionNode<Flat>) -> CompositionNodeId {
        let id = CompositionNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    fn root(&self) -> CompositionNodeId {
        CompositionNodeId(
            self.nodes
                .len()
                .checked_sub(1)
                .expect("composition must have a root"),
        )
    }

    fn validate_tree(&self) {
        let mut parents = vec![0; self.nodes.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            let mut child = |id: CompositionNodeId| {
                assert!(
                    id.index() < index,
                    "composition children must precede parents"
                );
                parents[id.index()] += 1;
                assert_eq!(
                    parents[id.index()],
                    1,
                    "composition leaves cannot share mutable state"
                );
            };
            match node {
                SelectorCompositionNode::Flat(_) => {}
                SelectorCompositionNode::Limited { selector, .. } => child(*selector),
                SelectorCompositionNode::Union { children, .. } => {
                    for &id in children {
                        child(id);
                    }
                }
                SelectorCompositionNode::Cartesian { left, right, .. } => {
                    child(*left);
                    child(*right);
                }
            }
        }
        assert!(
            parents[..self.root().index()]
                .iter()
                .all(|&count| count == 1),
            "composition must have one connected root"
        );
    }
}

impl<S, M, Flat: Debug, FlatState> Debug for SelectorComposition<S, M, Flat, FlatState> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectorComposition")
            .field("nodes", &self.nodes)
            .finish()
    }
}
