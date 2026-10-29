//! Public dynamic nearby-swap facade over the canonical scalar leaf.

use crate::heuristic::selector::scalar_neighborhood::spec::ScalarNeighborhoodBindingError;
use crate::heuristic::selector::scalar_neighborhood::spec::ScalarNeighborhoodSpec;
use std::fmt::{self, Debug};

use solverforge_core::domain::{DynamicScalarVariableSlot, PlanningSolution};
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{DynamicScalarSwapMove, MoveArena};

use super::move_selector::{MoveCursor, MoveSelector, MoveStreamContext};
use super::scalar_neighborhood::{
    dynamic_slot, emit_dynamic_scalar_swap_move, RuntimeScalarFacadeCursor, ScalarNeighborhoodLeaf,
};

/// Direct public facade for one declared dynamic nearby-entity source.
///
/// Missing source metadata is a structural binding failure. In particular, it
/// never changes the candidate universe to all entity pairs.
pub(crate) struct DynamicScalarNearbySwapMoveSelector<S> {
    leaf: ScalarNeighborhoodLeaf<S>,
}

pub(crate) type DynamicScalarNearbySwapMoveCursor<S> =
    RuntimeScalarFacadeCursor<S, DynamicScalarSwapMove<S>>;

impl<S> DynamicScalarNearbySwapMoveSelector<S>
where
    S: PlanningSolution,
{
    pub(crate) fn new(
        slot: DynamicScalarVariableSlot<S>,
        max_nearby: usize,
    ) -> Result<Self, ScalarNeighborhoodBindingError> {
        Ok(Self {
            leaf: ScalarNeighborhoodLeaf::from_spec(
                ScalarNeighborhoodSpec::NearbySwap { max_nearby },
                dynamic_slot(slot),
                None,
            )?,
        })
    }
}

impl<S> Debug for DynamicScalarNearbySwapMoveSelector<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DynamicScalarNearbySwapMoveSelector")
            .field("leaf", &self.leaf)
            .finish()
    }
}

impl<S> MoveSelector<S, DynamicScalarSwapMove<S>> for DynamicScalarNearbySwapMoveSelector<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    type Cursor<'a>
        = DynamicScalarNearbySwapMoveCursor<S>
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
            emit_dynamic_scalar_swap_move::<S>,
        )
    }

    fn size<D: Director<S>>(&self, director: &D) -> usize {
        let solution = director.working_solution();
        let mut cursor = self.open_cursor(director);
        let mut count = 0;
        while cursor.next_candidate(solution).is_some() {
            count += 1;
        }
        count
    }

    fn append_moves<D: Director<S>>(
        &self,
        director: &D,
        arena: &mut MoveArena<DynamicScalarSwapMove<S>>,
    ) {
        let solution = director.working_solution();
        let mut cursor = self.open_cursor(director);
        while let Some(id) = cursor.next_candidate(solution) {
            arena.push(cursor.take_candidate(id));
        }
    }
}
