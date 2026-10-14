//! Indexed mutable cursor arena over one immutable declaration arena.

use super::cartesian::SelectorCompositionCartesianCursor;
use super::{
    CompositionNodeId, SelectorComposition, SelectorCompositionNode, SequentialMoveCarrier,
    StatefulComposedFlat,
};
use crate::heuristic::selector::decorator::{
    resolve_union_weights, union_child_context, LimitedCursorBudget,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveStreamContext, ResourceMoveCursor,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
mod union;
use union::SelectorCompositionUnionCursor;

/// Retained leaf states use declaration coordinates. Structural nodes own no
/// duplicate state graph; their edges remain in the immutable declaration.
pub(super) struct SelectorCompositionStreamState<FlatState> {
    pub(super) leaves: Vec<Option<FlatState>>,
}

pub(super) fn new_stream_state<S, M, Flat, FlatState, Resources>(
    selector: &SelectorComposition<S, M, Flat, FlatState>,
) -> SelectorCompositionStreamState<FlatState>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    selector.validate_tree();
    SelectorCompositionStreamState {
        leaves: selector
            .nodes
            .iter()
            .map(|node| match node {
                SelectorCompositionNode::Flat(flat) => Some(flat.new_stream_state()),
                _ => None,
            })
            .collect(),
    }
}

#[allow(clippy::large_enum_variant)]
enum CursorNode<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    Flat(Flat::Cursor<'a>),
    Limited {
        child: CompositionNodeId,
        budget: LimitedCursorBudget,
    },
    Union(SelectorCompositionUnionCursor),
    Cartesian(SelectorCompositionCartesianCursor<S, M>),
}

/// Only IDs connect nodes. Taking a node for a recursive pull gives exclusive
/// access to descendants without aliasing or storing self-references.
pub(crate) struct SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    selector: &'a SelectorComposition<S, M, Flat, FlatState>,
    stream_state: SelectorCompositionStreamState<FlatState>,
    nodes: Vec<Option<CursorNode<'a, S, M, Flat, FlatState, Resources>>>,
}

pub(super) fn open_cursor_with_owned_stream_state<'a, S, M, Flat, FlatState, Resources, D>(
    selector: &'a SelectorComposition<S, M, Flat, FlatState>,
    stream_state: SelectorCompositionStreamState<FlatState>,
    resources: &mut Resources,
    director: &D,
    context: MoveStreamContext,
) -> SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
    D: Director<S>,
{
    let mut cursor = SelectorCompositionCursor {
        selector,
        stream_state,
        nodes: (0..selector.nodes.len()).map(|_| None).collect(),
    };
    cursor.open_node(selector.root(), resources, director, context);
    cursor
}

impl<'a, S, M, Flat, FlatState, Resources>
    SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    pub(super) fn into_stream_state(self) -> SelectorCompositionStreamState<FlatState> {
        // Drop live cursors before transferring retained states back to their owner.
        drop(self.nodes);
        self.stream_state
    }

    pub(super) fn open_node<D: Director<S>>(
        &mut self,
        id: CompositionNodeId,
        resources: &mut Resources,
        director: &D,
        context: MoveStreamContext,
    ) {
        assert!(
            self.nodes[id.index()].is_none(),
            "composition node already open"
        );
        let node = match &self.selector.nodes[id.index()] {
            SelectorCompositionNode::Flat(flat) => CursorNode::Flat(
                flat.open_cursor_with_stream_state(
                    self.stream_state.leaves[id.index()]
                        .as_mut()
                        .expect("flat state must exist"),
                    resources,
                    director,
                    context,
                ),
            ),
            SelectorCompositionNode::Limited {
                selector,
                selected_count_limit,
            } => {
                let child = *selector;
                let limit = *selected_count_limit;
                self.open_node(child, resources, director, context);
                CursorNode::Limited {
                    child,
                    budget: LimitedCursorBudget::new(limit),
                }
            }
            SelectorCompositionNode::Union {
                selection_order,
                weighting,
                weights,
                children,
            } => {
                let child_context = union_child_context(*selection_order, context);
                let sizes = match weighting {
                    solverforge_config::UnionWeighting::CandidateCount => children
                        .iter()
                        .map(|&child| self.selector_size(child, director))
                        .collect(),
                    _ => vec![0; children.len()],
                };
                let union = SelectorCompositionUnionCursor::new(
                    children,
                    *selection_order,
                    context,
                    resolve_union_weights(*weighting, weights, &sizes),
                );
                for &child in children {
                    self.open_node(child, resources, director, child_context);
                }
                CursorNode::Union(union)
            }
            SelectorCompositionNode::Cartesian {
                left,
                right,
                require_hard_improvement,
            } => {
                let (left, right, require) = (*left, *right, *require_hard_improvement);
                self.validate_cursor(right, director);
                self.open_node(left, resources, director, context);
                CursorNode::Cartesian(SelectorCompositionCartesianCursor::new(
                    require, left, right, context,
                ))
            }
        };
        self.nodes[id.index()] = Some(node);
    }

    pub(super) fn close_node(&mut self, id: CompositionNodeId) {
        let Some(node) = self.nodes[id.index()].take() else {
            return;
        };
        match node {
            CursorNode::Flat(_) => {}
            CursorNode::Limited { child, .. } => self.close_node(child),
            CursorNode::Union(_) => {
                let SelectorCompositionNode::Union { children, .. } =
                    &self.selector.nodes[id.index()]
                else {
                    unreachable!()
                };
                for &child in children {
                    self.close_node(child);
                }
            }
            CursorNode::Cartesian(cursor) => {
                self.close_node(cursor.left());
                self.close_node(cursor.right());
            }
        }
    }

    fn selector_size<D: Director<S>>(&self, id: CompositionNodeId, director: &D) -> usize {
        match &self.selector.nodes[id.index()] {
            SelectorCompositionNode::Flat(flat) => flat.size(director),
            SelectorCompositionNode::Limited {
                selector,
                selected_count_limit,
            } => self
                .selector_size(*selector, director)
                .min(*selected_count_limit),
            SelectorCompositionNode::Union { children, .. } => children
                .iter()
                .map(|&child| self.selector_size(child, director))
                .sum(),
            SelectorCompositionNode::Cartesian { left, right, .. } => self
                .selector_size(*left, director)
                .saturating_mul(self.selector_size(*right, director)),
        }
    }

    fn validate_cursor<D: Director<S>>(&self, id: CompositionNodeId, director: &D) {
        match &self.selector.nodes[id.index()] {
            SelectorCompositionNode::Flat(flat) => flat.validate_cursor(director),
            SelectorCompositionNode::Limited { selector, .. } => {
                self.validate_cursor(*selector, director)
            }
            SelectorCompositionNode::Union { children, .. } => {
                for &child in children {
                    self.validate_cursor(child, director);
                }
            }
            SelectorCompositionNode::Cartesian { left, right, .. } => {
                self.validate_cursor(*left, director);
                self.validate_cursor(*right, director);
            }
        }
    }

    pub(super) fn next_at<D: Director<S>>(
        &mut self,
        node_id: CompositionNodeId,
        director: &mut D,
        resources: &mut Resources,
    ) -> Option<CandidateId> {
        let mut node = self.nodes[node_id.index()]
            .take()
            .expect("composition cursor must be open");
        let result = match &mut node {
            CursorNode::Flat(cursor) => cursor.next_candidate_with_resources(director, resources),
            CursorNode::Limited { child, budget } => {
                if !budget.can_pull() {
                    None
                } else {
                    let result = self.next_at(*child, director, resources);
                    if result.is_some() {
                        budget.record_pull();
                    }
                    result
                }
            }
            CursorNode::Union(cursor) => {
                let SelectorCompositionNode::Union { children, .. } =
                    &self.selector.nodes[node_id.index()]
                else {
                    unreachable!()
                };
                cursor.next(children, |child| self.next_at(child, director, resources))
            }
            CursorNode::Cartesian(cursor) => cursor.next(self, director, resources),
        };
        self.nodes[node_id.index()] = Some(node);
        result
    }

    pub(super) fn candidate_at(
        &self,
        node: CompositionNodeId,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, M>> {
        match self.nodes[node.index()].as_ref()? {
            CursorNode::Flat(cursor) => cursor.candidate(id),
            CursorNode::Limited { child, .. } => self.candidate_at(*child, id),
            CursorNode::Union(cursor) => {
                let (child, id) = cursor.coordinate(id)?;
                self.candidate_at(child, id)
            }
            CursorNode::Cartesian(cursor) => cursor.candidate(id),
        }
    }

    pub(super) fn take_at(&mut self, node_id: CompositionNodeId, id: CandidateId) -> M {
        let mut node = self.nodes[node_id.index()]
            .take()
            .expect("composition cursor must be open");
        let result = match &mut node {
            CursorNode::Flat(cursor) => cursor.take_candidate(id),
            CursorNode::Limited { child, .. } => self.take_at(*child, id),
            CursorNode::Union(cursor) => {
                let (child, id) = cursor
                    .coordinate(id)
                    .expect("selected union candidate must exist");
                self.take_at(child, id)
            }
            CursorNode::Cartesian(cursor) => cursor.take(self, id),
        };
        self.nodes[node_id.index()] = Some(node);
        result
    }

    pub(super) fn release_at(&mut self, node_id: CompositionNodeId, id: CandidateId) -> bool {
        let mut node = self.nodes[node_id.index()]
            .take()
            .expect("composition cursor must be open");
        let result = match &mut node {
            CursorNode::Flat(cursor) => cursor.release_candidate(id),
            CursorNode::Limited { child, .. } => self.release_at(*child, id),
            CursorNode::Union(cursor) => cursor
                .coordinate(id)
                .is_some_and(|(child, id)| self.release_at(child, id)),
            CursorNode::Cartesian(cursor) => cursor.release(id),
        };
        self.nodes[node_id.index()] = Some(node);
        result
    }

    fn apply_at<D: Director<S>>(
        &mut self,
        node_id: CompositionNodeId,
        id: CandidateId,
        director: &mut D,
    ) {
        let mut node = self.nodes[node_id.index()]
            .take()
            .expect("composition cursor must be open");
        match &mut node {
            CursorNode::Flat(cursor) => cursor.apply_owned_candidate(id, director),
            CursorNode::Limited { child, .. } => self.apply_at(*child, id, director),
            CursorNode::Union(cursor) => {
                let (child, id) = cursor
                    .coordinate(id)
                    .expect("selected union candidate must exist");
                self.apply_at(child, id, director);
            }
            CursorNode::Cartesian(cursor) => {
                cursor.take(self, id).do_move(director);
            }
        }
        self.nodes[node_id.index()] = Some(node);
    }

    fn selector_index_at(&self, node: CompositionNodeId, id: CandidateId) -> Option<usize> {
        match self.nodes[node.index()].as_ref()? {
            CursorNode::Flat(cursor) => cursor.selector_index(id),
            CursorNode::Limited { child, .. } => self.selector_index_at(*child, id),
            CursorNode::Union(cursor) => cursor.selector_index(id),
            CursorNode::Cartesian(_) => None,
        }
    }
}

impl<S, M, Flat, FlatState, Resources> ResourceMoveCursor<S, M, Resources>
    for SelectorCompositionCursor<'_, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    fn next_candidate_with_resources<D: Director<S>>(
        &mut self,
        director: &mut D,
        resources: &mut Resources,
    ) -> Option<CandidateId> {
        self.next_at(self.selector.root(), director, resources)
    }
    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        self.candidate_at(self.selector.root(), id)
    }
    fn take_candidate(&mut self, id: CandidateId) -> M {
        self.take_at(self.selector.root(), id)
    }
    fn apply_owned_candidate<D: Director<S>>(&mut self, id: CandidateId, director: &mut D) {
        self.apply_at(self.selector.root(), id, director);
    }
    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.release_at(self.selector.root(), id)
    }
    fn selector_index(&self, id: CandidateId) -> Option<usize> {
        self.selector_index_at(self.selector.root(), id)
    }
}
