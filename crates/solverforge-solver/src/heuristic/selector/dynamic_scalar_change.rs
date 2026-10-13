//! Public dynamic scalar change facade over the canonical scalar leaf.

use std::fmt::{self, Debug};

use solverforge_core::domain::{DynamicScalarVariableSlot, PlanningSolution};
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{DynamicScalarChangeMove, MoveArena};

use super::move_selector::{MoveCursor, MoveSelector, MoveStreamContext};
use super::scalar_neighborhood::{
    dynamic_slot, emit_dynamic_scalar_change_move, RuntimeScalarFacadeCursor,
    ScalarNeighborhoodLeaf, ScalarNeighborhoodSpec,
};

/// Direct public facade. Candidate discovery, ordering, and ownership live in
/// `ScalarNeighborhoodLeaf`; this type preserves the established public move
/// payload only.
pub struct DynamicScalarChangeMoveSelector<S> {
    leaf: ScalarNeighborhoodLeaf<S>,
}

pub type DynamicScalarChangeMoveCursor<S> =
    RuntimeScalarFacadeCursor<S, DynamicScalarChangeMove<S>>;

impl<S> DynamicScalarChangeMoveSelector<S>
where
    S: PlanningSolution,
{
    pub fn new(slot: DynamicScalarVariableSlot<S>, value_candidate_limit: Option<usize>) -> Self {
        let leaf = ScalarNeighborhoodLeaf::from_spec(
            ScalarNeighborhoodSpec::Change {
                value_candidate_limit,
            },
            dynamic_slot(slot),
            None,
        )
        .expect("ordinary dynamic scalar slots always provide a candidate stream");
        Self { leaf }
    }
}

impl<S> Debug for DynamicScalarChangeMoveSelector<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DynamicScalarChangeMoveSelector")
            .field("leaf", &self.leaf)
            .finish()
    }
}

impl<S> MoveSelector<S, DynamicScalarChangeMove<S>> for DynamicScalarChangeMoveSelector<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    type Cursor<'a>
        = DynamicScalarChangeMoveCursor<S>
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
        arena: &mut MoveArena<DynamicScalarChangeMove<S>>,
    ) {
        let solution = director.working_solution();
        let mut cursor = self.open_cursor(director);
        while let Some(id) = cursor.next_candidate(solution) {
            arena.push(cursor.take_candidate(id));
        }
    }
}
