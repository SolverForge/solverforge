//! Public dynamic nearby-change facade over the canonical scalar leaf.

use std::fmt::{self, Debug};

use solverforge_core::domain::{DynamicScalarVariableSlot, PlanningSolution};
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{DynamicScalarChangeMove, MoveArena};

use super::move_selector::{MoveSelector, MoveStreamContext};
use super::scalar_neighborhood::{
    dynamic_slot, emit_dynamic_scalar_change_move, RuntimeScalarFacadeCursor,
    ScalarNeighborhoodBindingError, ScalarNeighborhoodLeaf, ScalarNeighborhoodSpec,
};

/// Direct public facade for one declared dynamic nearby-value source.
///
/// Construction is fallible by design. A missing structural nearby source is
/// a binding error, not permission to substitute ordinary candidates.
pub struct DynamicScalarNearbyChangeMoveSelector<S> {
    leaf: ScalarNeighborhoodLeaf<S>,
}

pub type DynamicScalarNearbyChangeMoveCursor<S> =
    RuntimeScalarFacadeCursor<S, DynamicScalarChangeMove<S>>;

impl<S> DynamicScalarNearbyChangeMoveSelector<S>
where
    S: PlanningSolution,
{
    pub fn new(
        slot: DynamicScalarVariableSlot<S>,
        max_nearby: usize,
        value_candidate_limit: Option<usize>,
    ) -> Result<Self, ScalarNeighborhoodBindingError> {
        Ok(Self {
            leaf: ScalarNeighborhoodLeaf::from_spec(
                ScalarNeighborhoodSpec::NearbyChange {
                    max_nearby,
                    value_candidate_limit,
                },
                dynamic_slot(slot),
                None,
            )?,
        })
    }
}

impl<S> Debug for DynamicScalarNearbyChangeMoveSelector<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DynamicScalarNearbyChangeMoveSelector")
            .field("leaf", &self.leaf)
            .finish()
    }
}

impl<S> MoveSelector<S, DynamicScalarChangeMove<S>> for DynamicScalarNearbyChangeMoveSelector<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    type Cursor<'a>
        = DynamicScalarNearbyChangeMoveCursor<S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let mut stream_state = self.leaf.new_stream_state();
        RuntimeScalarFacadeCursor::new(
            self.leaf
                .open_cursor_with_stream_state(&mut stream_state, director, context),
            emit_dynamic_scalar_change_move::<S>,
        )
    }

    fn size<D: Director<S>>(&self, director: &D) -> usize {
        self.open_cursor(director).count()
    }

    fn append_moves<D: Director<S>>(
        &self,
        director: &D,
        arena: &mut MoveArena<DynamicScalarChangeMove<S>>,
    ) {
        arena.extend(self.open_cursor(director));
    }
}
