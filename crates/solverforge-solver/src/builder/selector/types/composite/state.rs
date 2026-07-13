//! Recursive stream-state ownership for the compiled selector graph.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::{
    resolve_union_weights, union_child_context, LimitedMoveCursor,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveStreamContext, ResourceMoveCursor,
};

use super::{
    SelectorComposition, SelectorCompositionCartesianCursor, SequentialMoveCarrier,
    StatefulComposedFlat,
};

mod union;

use union::SelectorCompositionUnionCursor;

/// The recursive state tree mirrors one frozen composition tree exactly.
///
/// One `Flat` or `Limited` node owns one state per frozen leaf. The tree moves
/// into a live cursor and returns to its execution owner when that cursor
/// drops, preserving stateful leaf streams across cursor-open boundaries.
pub(super) enum SelectorCompositionStreamState<FlatState> {
    Flat(FlatState),
    Limited(Box<SelectorCompositionStreamState<FlatState>>),
    Cartesian {
        left: Box<SelectorCompositionStreamState<FlatState>>,
        right: Box<SelectorCompositionStreamState<FlatState>>,
    },
    Union(Vec<SelectorCompositionStreamState<FlatState>>),
}

pub(super) fn new_stream_state<S, M, Flat, FlatState, Resources>(
    selector: &SelectorComposition<S, M, Flat, FlatState>,
) -> SelectorCompositionStreamState<FlatState>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    match selector {
        SelectorComposition::Flat(selector) => {
            SelectorCompositionStreamState::Flat(selector.new_stream_state())
        }
        SelectorComposition::Limited { selector, .. } => {
            SelectorCompositionStreamState::Limited(Box::new(new_stream_state(selector)))
        }
        SelectorComposition::Union { children, .. } => {
            SelectorCompositionStreamState::Union(children.iter().map(new_stream_state).collect())
        }
        SelectorComposition::Cartesian(selector) => SelectorCompositionStreamState::Cartesian {
            left: Box::new(new_stream_state(&selector.left)),
            right: Box::new(new_stream_state(&selector.right)),
        },
    }
}

pub(super) fn open_cursor_with_owned_stream_state<'a, S, M, Flat, FlatState, Resources, D>(
    selector: &'a SelectorComposition<S, M, Flat, FlatState>,
    stream_state: SelectorCompositionStreamState<FlatState>,
    resources: &mut Resources,
    score_director: &D,
    context: MoveStreamContext,
) -> SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
    D: Director<S>,
{
    match (selector, stream_state) {
        (SelectorComposition::Flat(selector), SelectorCompositionStreamState::Flat(mut state)) => {
            let cursor = selector.open_cursor_with_stream_state(
                &mut state,
                resources,
                score_director,
                context,
            );
            SelectorCompositionCursor::Flat {
                cursor,
                stream_state: state,
            }
        }
        (
            SelectorComposition::Limited {
                selector,
                selected_count_limit,
            },
            SelectorCompositionStreamState::Limited(state),
        ) => {
            let cursor = open_cursor_with_owned_stream_state(
                selector,
                *state,
                resources,
                score_director,
                context,
            );
            SelectorCompositionCursor::Limited {
                cursor: Box::new(LimitedMoveCursor::new(cursor, *selected_count_limit)),
            }
        }
        (
            SelectorComposition::Union {
                selection_order,
                weighting,
                weights,
                children,
            },
            SelectorCompositionStreamState::Union(stream_states),
        ) => {
            assert_eq!(
                children.len(),
                stream_states.len(),
                "recursive union stream state must match its frozen children"
            );
            let child_context = union_child_context(*selection_order, context);
            let child_sizes = match weighting {
                solverforge_config::UnionWeighting::CandidateCount => children
                    .iter()
                    .map(|child| selector_size(child, score_director))
                    .collect(),
                solverforge_config::UnionWeighting::Equal
                | solverforge_config::UnionWeighting::Fixed => vec![0; children.len()],
            };
            let resolved_weights = resolve_union_weights(*weighting, weights, &child_sizes);
            let cursors = children
                .iter()
                .zip(stream_states)
                .map(|(child, child_state)| {
                    open_cursor_with_owned_stream_state(
                        child,
                        child_state,
                        resources,
                        score_director,
                        child_context,
                    )
                })
                .collect();
            SelectorCompositionCursor::Union(SelectorCompositionUnionCursor::new(
                cursors,
                *selection_order,
                context,
                resolved_weights,
            ))
        }
        (
            SelectorComposition::Cartesian(selector),
            SelectorCompositionStreamState::Cartesian { left, right },
        ) => {
            // Validate at Cartesian open as before, but defer actual right
            // cursor opening until a legal left preview has been applied.
            validate_cursor(&selector.right, score_director);
            let left_cursor = open_cursor_with_owned_stream_state(
                &selector.left,
                *left,
                resources,
                score_director,
                context,
            );
            SelectorCompositionCursor::Cartesian(SelectorCompositionCartesianCursor::new(
                selector.require_hard_improvement,
                left_cursor,
                &selector.right,
                *right,
                score_director,
                context,
            ))
        }
        _ => panic!("selector composition stream state must match its frozen tree"),
    }
}

fn selector_size<S, M, Flat, FlatState, Resources, D>(
    selector: &SelectorComposition<S, M, Flat, FlatState>,
    score_director: &D,
) -> usize
where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
    D: Director<S>,
{
    match selector {
        SelectorComposition::Flat(selector) => selector.size(score_director),
        SelectorComposition::Limited {
            selector,
            selected_count_limit,
        } => selector_size(selector, score_director).min(*selected_count_limit),
        SelectorComposition::Union { children, .. } => children
            .iter()
            .map(|child| selector_size(child, score_director))
            .sum(),
        SelectorComposition::Cartesian(selector) => selector_size(&selector.left, score_director)
            .saturating_mul(selector_size(&selector.right, score_director)),
    }
}

fn validate_cursor<S, M, Flat, FlatState, Resources, D>(
    selector: &SelectorComposition<S, M, Flat, FlatState>,
    score_director: &D,
) where
    S: PlanningSolution,
    M: SequentialMoveCarrier<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
    D: Director<S>,
{
    match selector {
        SelectorComposition::Flat(selector) => selector.validate_cursor(score_director),
        SelectorComposition::Limited { selector, .. } => validate_cursor(selector, score_director),
        SelectorComposition::Union { children, .. } => {
            for child in children {
                validate_cursor(child, score_director);
            }
        }
        SelectorComposition::Cartesian(selector) => {
            validate_cursor(&selector.left, score_director);
            validate_cursor(&selector.right, score_director);
        }
    }
}

/// Cursor returned by the one resource-aware recursive composition path.
///
/// It owns state while open. Leaf cursor state borrows end at leaf open; a
/// Cartesian node moves a right state subtree into its deferred cursor and
/// moves it back when that row closes. The resource is deliberately not held
/// here: the solve-owned execution lends it only to the next reachable pull.
#[allow(clippy::large_enum_variant)]
pub(crate) enum SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources> + 'a,
{
    Flat {
        cursor: Flat::Cursor<'a>,
        stream_state: FlatState,
    },
    Limited {
        cursor:
            Box<LimitedMoveCursor<SelectorCompositionCursor<'a, S, M, Flat, FlatState, Resources>>>,
    },
    Union(SelectorCompositionUnionCursor<'a, S, M, Flat, FlatState, Resources>),
    Cartesian(SelectorCompositionCartesianCursor<'a, S, M, Flat, FlatState, Resources>),
}

impl<S, M, Flat, FlatState, Resources>
    SelectorCompositionCursor<'_, S, M, Flat, FlatState, Resources>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: StatefulComposedFlat<S, M, FlatState, Resources>,
{
    pub(super) fn into_stream_state(self) -> SelectorCompositionStreamState<FlatState>
    where
        M: SequentialMoveCarrier<S>,
    {
        match self {
            Self::Flat { stream_state, .. } => SelectorCompositionStreamState::Flat(stream_state),
            Self::Limited { cursor } => SelectorCompositionStreamState::Limited(Box::new(
                cursor.into_inner().into_stream_state(),
            )),
            Self::Union(cursor) => cursor.into_stream_state(),
            Self::Cartesian(cursor) => cursor.into_stream_state(),
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
    fn next_candidate_with_resources(&mut self, resources: &mut Resources) -> Option<CandidateId> {
        match self {
            Self::Flat { cursor, .. } => cursor.next_candidate_with_resources(resources),
            Self::Limited { cursor, .. } => cursor.next_candidate_with_resources(resources),
            Self::Union(cursor) => cursor.next_candidate_with_resources(resources),
            Self::Cartesian(cursor) => cursor.next_candidate_with_resources(resources),
        }
    }

    fn candidate(&self, index: CandidateId) -> Option<MoveCandidateRef<'_, S, M>> {
        match self {
            Self::Flat { cursor, .. } => cursor.candidate(index),
            Self::Limited { cursor, .. } => cursor.candidate(index),
            Self::Union(cursor) => cursor.candidate(index),
            Self::Cartesian(cursor) => cursor.candidate(index),
        }
    }

    fn take_candidate(&mut self, index: CandidateId) -> M {
        match self {
            Self::Flat { cursor, .. } => cursor.take_candidate(index),
            Self::Limited { cursor, .. } => cursor.take_candidate(index),
            Self::Union(cursor) => cursor.take_candidate(index),
            Self::Cartesian(cursor) => cursor.take_candidate(index),
        }
    }

    fn apply_owned_candidate<D: Director<S>>(
        &mut self,
        index: CandidateId,
        score_director: &mut D,
    ) {
        match self {
            Self::Flat { cursor, .. } => cursor.apply_owned_candidate(index, score_director),
            Self::Limited { cursor, .. } => cursor.apply_owned_candidate(index, score_director),
            Self::Union(cursor) => cursor.apply_owned_candidate(index, score_director),
            Self::Cartesian(cursor) => cursor.apply_owned_candidate(index, score_director),
        }
    }

    fn release_candidate(&mut self, index: CandidateId) -> bool {
        match self {
            Self::Flat { cursor, .. } => cursor.release_candidate(index),
            Self::Limited { cursor, .. } => cursor.release_candidate(index),
            Self::Union(cursor) => cursor.release_candidate(index),
            Self::Cartesian(cursor) => cursor.release_candidate(index),
        }
    }

    fn selector_index(&self, index: CandidateId) -> Option<usize> {
        match self {
            Self::Flat { cursor, .. } => cursor.selector_index(index),
            Self::Limited { cursor, .. } => cursor.selector_index(index),
            Self::Union(cursor) => cursor.selector_index(index),
            Self::Cartesian(cursor) => cursor.selector_index(index),
        }
    }
}
