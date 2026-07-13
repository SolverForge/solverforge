//! One recursive selector-composition surface.
//!
//! The compiled runner supplies frozen leaves. This module owns recursive
//! Limited/Union/Cartesian state around them.

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

pub(crate) use cartesian::SelectorCompositionCartesianCursor;
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

/// One recursive node over a frozen flat leaf set.
///
/// `Flat`, `Limited`, `Union`, and `Cartesian` preserve one recursive
/// ownership and cursor path. A union therefore remains valid inside either
/// Cartesian child instead of being flattened into an unrelated outer path.
pub enum SelectorComposition<S, M, Flat, FlatState>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
{
    Flat(Flat),
    Limited {
        selector: Box<SelectorCompositionChild<S, M, Flat, FlatState>>,
        selected_count_limit: usize,
    },
    Union {
        selection_order: UnionSelectionOrder,
        weighting: UnionWeighting,
        weights: Vec<u64>,
        children: Vec<SelectorCompositionChild<S, M, Flat, FlatState>>,
    },
    Cartesian(SelectorCompositionCartesian<S, M, Flat, FlatState>),
}

/// A Cartesian child is the same recursive composition node.
pub type SelectorCompositionChild<S, M, Flat, FlatState> =
    SelectorComposition<S, M, Flat, FlatState>;

/// A recursive Cartesian node with deferred right-child opening.
pub struct SelectorCompositionCartesian<S, M, Flat, FlatState>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
{
    left: Box<SelectorCompositionChild<S, M, Flat, FlatState>>,
    right: Box<SelectorCompositionChild<S, M, Flat, FlatState>>,
    require_hard_improvement: bool,
    _marker: PhantomData<fn() -> (S, M, FlatState)>,
}

impl<S, M, Flat, FlatState> SelectorCompositionCartesian<S, M, Flat, FlatState>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
{
    pub fn new(
        left: SelectorCompositionChild<S, M, Flat, FlatState>,
        right: SelectorCompositionChild<S, M, Flat, FlatState>,
    ) -> Self {
        Self {
            left: Box::new(left),
            right: Box::new(right),
            require_hard_improvement: false,
            _marker: PhantomData,
        }
    }

    pub fn with_require_hard_improvement(mut self, require_hard_improvement: bool) -> Self {
        self.require_hard_improvement = require_hard_improvement;
        self
    }
}

impl<S, M, Flat, FlatState> Debug for SelectorCompositionCartesian<S, M, Flat, FlatState>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CartesianProductSelector")
            .field("left", &self.left)
            .field("right", &self.right)
            .field("require_hard_improvement", &self.require_hard_improvement)
            .finish()
    }
}

impl<S, M, Flat, FlatState> Debug for SelectorComposition<S, M, Flat, FlatState>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    Flat: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(selector) => write!(f, "Neighborhood::Flat({selector:?})"),
            Self::Limited {
                selector,
                selected_count_limit,
            } => f
                .debug_struct("Neighborhood::Limited")
                .field("selector", selector)
                .field("selected_count_limit", selected_count_limit)
                .finish(),
            Self::Union {
                selection_order,
                weighting,
                weights,
                children,
            } => f
                .debug_struct("Neighborhood::Union")
                .field("selection_order", selection_order)
                .field("weighting", weighting)
                .field("weights", weights)
                .field("children", children)
                .finish(),
            Self::Cartesian(selector) => write!(f, "Neighborhood::Cartesian({selector:?})"),
        }
    }
}
