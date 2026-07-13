//! One streamed cursor for every compiled runtime list neighborhood.

mod probe;
mod slot;

use std::fmt;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use solverforge_config::{SelectionOrder, UnionSelectionOrder};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::context::list_access::ListAccess;
use crate::heuristic::selector::decorator::VecUnionMoveCursor;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::seed::scoped_seed;

use self::slot::{open_slot_cursor, RuntimeListSlotCursor};
use super::spec::{RuntimeListNeighborhoodPlan, RuntimeListNeighborhoodSpec};
use super::RuntimeListMove;

/// A compiled list leaf ready to open a per-step cursor.
pub(crate) struct RuntimeListNeighborhoodSelector<S, V, DM, IDM> {
    plan: RuntimeListNeighborhoodPlan<S, V, DM, IDM>,
}

impl<S, V, DM, IDM> RuntimeListNeighborhoodSelector<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    pub(crate) fn new(plan: RuntimeListNeighborhoodPlan<S, V, DM, IDM>) -> Self {
        Self { plan }
    }

    pub(crate) fn plan(&self) -> &RuntimeListNeighborhoodPlan<S, V, DM, IDM> {
        &self.plan
    }

    /// Creates the leaf's mutable per-solve stream state. The compiled plan
    /// and cursor factory remain immutable; only ruin RNG progression lives
    /// in this object owned by the generic composed execution.
    pub(crate) fn new_stream_state(&self) -> RuntimeListNeighborhoodStreamState {
        RuntimeListNeighborhoodStreamState::new(&self.plan)
    }

    /// Opens through the state-owning composition boundary. The returned
    /// cursor owns derived ruin seeds and generated candidates, so it never
    /// borrows `stream_state` after this call returns.
    pub(crate) fn open_cursor_with_stream_state<'a, D: Director<S>>(
        &'a self,
        stream_state: &mut RuntimeListNeighborhoodStreamState,
        score_director: &D,
        context: MoveStreamContext,
    ) -> RuntimeListNeighborhoodCursor<'a, S, V, DM, IDM> {
        let ruin_seeds = match self.plan.spec {
            RuntimeListNeighborhoodSpec::Ruin { .. } => stream_state
                .ruin_rngs_mut(self.plan.slots.len())
                .iter_mut()
                .map(|rng| rng.random::<u64>() ^ context.offset_seed(0x7157_8011_C0DE_0001) as u64)
                .collect(),
            _ => Vec::new(),
        };
        let slot_cursors = self
            .plan
            .slots
            .iter()
            .cloned()
            .enumerate()
            .map(|(slot_index, slot)| {
                open_slot_cursor(
                    self.plan.spec,
                    slot,
                    score_director.working_solution(),
                    context,
                    &self.plan.kopt_patterns,
                    ruin_seeds.get(slot_index).copied(),
                )
            })
            .collect::<Vec<_>>();
        let slot_count = slot_cursors.len();
        let selection_order = match context.selection_order() {
            SelectionOrder::Random | SelectionOrder::Shuffled => {
                UnionSelectionOrder::StratifiedRandom
            }
            SelectionOrder::Original | SelectionOrder::Sorted | SelectionOrder::Probabilistic => {
                UnionSelectionOrder::Sequential
            }
        };
        RuntimeListNeighborhoodCursor {
            inner: VecUnionMoveCursor::new(
                slot_cursors,
                selection_order,
                context,
                vec![1; slot_count],
            ),
        }
    }
}

/// The only mutable state for one immutable compiled list leaf during a
/// solve. It is created once by the composed execution, not kept behind
/// interior mutability on the selector definition.
pub(crate) struct RuntimeListNeighborhoodStreamState {
    ruin_rngs: Vec<SmallRng>,
}

impl RuntimeListNeighborhoodStreamState {
    fn new<S, V, DM, IDM>(plan: &RuntimeListNeighborhoodPlan<S, V, DM, IDM>) -> Self
    where
        S: PlanningSolution + Clone + Send + Sync + 'static,
        V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
        DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
        IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    {
        let ruin_rngs = match plan.spec {
            RuntimeListNeighborhoodSpec::Ruin { .. } => plan
                .slots
                .iter()
                .map(|slot| {
                    let seed = scoped_seed(
                        plan.random_seed,
                        slot.descriptor_index(),
                        slot.variable_name(),
                        "list_ruin_move_selector",
                    );
                    match seed {
                        Some(seed) => SmallRng::seed_from_u64(seed),
                        None => SmallRng::from_rng(&mut rand::rng()),
                    }
                })
                .collect(),
            _ => Vec::new(),
        };
        Self { ruin_rngs }
    }

    fn ruin_rngs_mut(&mut self, expected_slots: usize) -> &mut [SmallRng] {
        assert_eq!(
            self.ruin_rngs.len(),
            expected_slots,
            "list ruin stream state must belong to its frozen leaf"
        );
        &mut self.ruin_rngs
    }
}

impl fmt::Debug for RuntimeListNeighborhoodStreamState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeListNeighborhoodStreamState")
            .field("ruin_stream_count", &self.ruin_rngs.len())
            .finish()
    }
}

impl<S, V, DM, IDM> fmt::Debug for RuntimeListNeighborhoodSelector<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeListNeighborhoodSelector")
            .field("kind", &self.plan.kind())
            .field("slot_count", &self.plan.slots().len())
            .finish_non_exhaustive()
    }
}

impl<S, V, DM, IDM> MoveSelector<S, RuntimeListMove<S, V, DM, IDM>>
    for RuntimeListNeighborhoodSelector<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Cursor<'a>
        = RuntimeListNeighborhoodCursor<'a, S, V, DM, IDM>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        if matches!(self.plan.spec, RuntimeListNeighborhoodSpec::Ruin { .. }) {
            panic!(
                "runtime list ruin leaves must be opened through their persistent composed stream state"
            );
        }
        let mut stream_state = self.new_stream_state();
        self.open_cursor_with_stream_state(&mut stream_state, score_director, context)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self.plan.spec {
            RuntimeListNeighborhoodSpec::Ruin { moves_per_step, .. } => self
                .plan
                .slots
                .iter()
                .filter(|slot| slot.entity_count(score_director.working_solution()) > 0)
                .count()
                .saturating_mul(moves_per_step),
            _ => self.open_cursor(score_director).count(),
        }
    }
}

/// Canonical union boundary for the streamed per-slot state machines.
pub(crate) struct RuntimeListNeighborhoodCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    inner: VecUnionMoveCursor<
        S,
        RuntimeListMove<S, V, DM, IDM>,
        RuntimeListSlotCursor<'a, S, V, DM, IDM>,
    >,
}

impl<S, V, DM, IDM> MoveCursor<S, RuntimeListMove<S, V, DM, IDM>>
    for RuntimeListNeighborhoodCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, RuntimeListMove<S, V, DM, IDM>>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> RuntimeListMove<S, V, DM, IDM> {
        self.inner.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<RuntimeListMove<S, V, DM, IDM>> {
        self.inner.next_owned_candidate()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'b> fn(MoveCandidateRef<'b, S, RuntimeListMove<S, V, DM, IDM>>) -> bool,
    ) -> Option<RuntimeListMove<S, V, DM, IDM>> {
        self.inner.next_owned_candidate_matching(predicate)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V, DM, IDM> Iterator for RuntimeListNeighborhoodCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Item = RuntimeListMove<S, V, DM, IDM>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}
