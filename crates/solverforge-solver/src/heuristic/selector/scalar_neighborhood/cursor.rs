mod change;
mod pillar;
mod ruin;
mod swap;

use std::fmt;

use rand::rngs::SmallRng;
use rand::SeedableRng;
use solverforge_config::MoveSelectorConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::RuntimeScalarSlot;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use crate::heuristic::selector::seed::scoped_seed;

use super::r#move::RuntimeScalarMove;
use super::spec::{
    RuntimeScalarRecipe, ScalarNeighborhoodBindingError, ScalarNeighborhoodKind,
    ScalarNeighborhoodSpec,
};

use change::{ChangeCursor, NearbyChangeCursor};
use pillar::{PillarChangeCursor, PillarSwapCursor};
use ruin::{generate_ruin_batches, RuinRecreateCursor};
use swap::{NearbySwapCursor, SwapCursor};

/// One frozen scalar leaf. The generic composed tree owns composition and
/// per-solve state; this type only opens one family cursor for one slot.
pub(crate) struct ScalarNeighborhoodLeaf<S> {
    spec: ScalarNeighborhoodSpec,
    slot: RuntimeScalarSlot<S>,
    random_seed: Option<u64>,
}

impl<S> ScalarNeighborhoodLeaf<S>
where
    S: PlanningSolution,
{
    pub(crate) fn new(
        kind: ScalarNeighborhoodKind,
        config: &MoveSelectorConfig,
        slot: RuntimeScalarSlot<S>,
        random_seed: Option<u64>,
    ) -> Result<Self, ScalarNeighborhoodBindingError> {
        let spec = ScalarNeighborhoodSpec::from_config(kind, config)?;
        Self::from_spec(spec, slot, random_seed)
    }

    pub(crate) fn from_spec(
        spec: ScalarNeighborhoodSpec,
        slot: RuntimeScalarSlot<S>,
        random_seed: Option<u64>,
    ) -> Result<Self, ScalarNeighborhoodBindingError> {
        ScalarNeighborhoodBindingError::validate_slot(spec.kind(), &slot)?;
        Ok(Self {
            spec,
            slot,
            random_seed,
        })
    }

    pub(crate) fn new_stream_state(&self) -> ScalarNeighborhoodStreamState {
        match self.spec {
            ScalarNeighborhoodSpec::RuinRecreate { .. } => ScalarNeighborhoodStreamState::Ruin(
                ScalarRuinStreamState::new(self.random_seed, &self.slot),
            ),
            _ => ScalarNeighborhoodStreamState::Stateless,
        }
    }

    /// The exact future generic-composer hook. The mutable state is consumed
    /// only while an RRC batch is generated; the returned cursor owns all
    /// batches and never borrows state.
    pub(crate) fn open_cursor_with_stream_state<D>(
        &self,
        state: &mut ScalarNeighborhoodStreamState,
        director: &D,
        context: MoveStreamContext,
    ) -> RuntimeScalarNeighborhoodCursor<S>
    where
        D: Director<S>,
        S::Score: Score,
    {
        let solution = director.clone_working_solution();
        let state = match self.spec {
            ScalarNeighborhoodSpec::Change {
                value_candidate_limit,
            } => RuntimeScalarCursorState::Change(ChangeCursor::new(
                self.slot.clone(),
                solution,
                context,
                value_candidate_limit,
            )),
            ScalarNeighborhoodSpec::Swap => RuntimeScalarCursorState::Swap(SwapCursor::new(
                self.slot.clone(),
                solution,
                context,
            )),
            ScalarNeighborhoodSpec::NearbyChange {
                max_nearby,
                value_candidate_limit,
            } => RuntimeScalarCursorState::NearbyChange(NearbyChangeCursor::new(
                self.slot.clone(),
                solution,
                context,
                max_nearby,
                value_candidate_limit.unwrap_or(usize::MAX),
            )),
            ScalarNeighborhoodSpec::NearbySwap { max_nearby } => {
                RuntimeScalarCursorState::NearbySwap(NearbySwapCursor::new(
                    self.slot.clone(),
                    solution,
                    context,
                    max_nearby,
                ))
            }
            ScalarNeighborhoodSpec::PillarChange {
                minimum_sub_pillar_size,
                maximum_sub_pillar_size,
                value_candidate_limit,
            } => RuntimeScalarCursorState::PillarChange(PillarChangeCursor::new(
                self.slot.clone(),
                solution,
                context,
                minimum_sub_pillar_size,
                maximum_sub_pillar_size,
                value_candidate_limit,
            )),
            ScalarNeighborhoodSpec::PillarSwap {
                minimum_sub_pillar_size,
                maximum_sub_pillar_size,
            } => RuntimeScalarCursorState::PillarSwap(PillarSwapCursor::new(
                self.slot.clone(),
                solution,
                context,
                minimum_sub_pillar_size,
                maximum_sub_pillar_size,
            )),
            ScalarNeighborhoodSpec::RuinRecreate {
                min_ruin_count,
                max_ruin_count,
                moves_per_step,
                value_candidate_limit,
                recreate_heuristic_type,
            } => {
                let ScalarNeighborhoodStreamState::Ruin(ruin_state) = state else {
                    panic!("ruin/recreate scalar leaf requires its matching mutable stream state");
                };
                let mut batches = generate_ruin_batches(
                    &solution,
                    &self.slot,
                    min_ruin_count,
                    max_ruin_count,
                    moves_per_step,
                    ruin_state.rng_mut(),
                );
                context.apply_selection_order(
                    &mut batches,
                    0x5CA1_A2C0_7A11_0001 ^ slot_identity(&self.slot),
                );
                RuntimeScalarCursorState::RuinRecreate(RuinRecreateCursor::new(
                    self.slot.clone(),
                    solution,
                    batches,
                    value_candidate_limit,
                    recreate_heuristic_type,
                ))
            }
        };
        RuntimeScalarNeighborhoodCursor {
            store: CandidateStore::new(),
            state,
        }
    }
}

impl<S> fmt::Debug for ScalarNeighborhoodLeaf<S>
where
    S: PlanningSolution,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ScalarNeighborhoodLeaf")
            .field("kind", &self.spec.kind())
            .field("slot", &self.slot.id())
            .finish()
    }
}

impl<S> MoveSelector<S, RuntimeScalarMove<S>> for ScalarNeighborhoodLeaf<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    type Cursor<'a>
        = RuntimeScalarNeighborhoodCursor<S>
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
        if matches!(self.spec, ScalarNeighborhoodSpec::RuinRecreate { .. }) {
            panic!(
                "ruin/recreate scalar leaves must be opened through their persistent composed stream state"
            );
        }
        let mut state = Self::new_stream_state(self);
        Self::open_cursor_with_stream_state(self, &mut state, director, context)
    }

    fn size<D: Director<S>>(&self, director: &D) -> usize {
        match self.spec {
            ScalarNeighborhoodSpec::RuinRecreate { moves_per_step, .. } => {
                usize::from(self.slot.entity_count(director.working_solution()) > 0)
                    .saturating_mul(moves_per_step)
            }
            _ => self.open_cursor(director).count(),
        }
    }
}

/// Solve-owned random stream for exactly one ruin/recreate leaf.
pub(crate) struct ScalarRuinStreamState {
    rng: SmallRng,
}

impl ScalarRuinStreamState {
    fn new<S>(random_seed: Option<u64>, slot: &RuntimeScalarSlot<S>) -> Self {
        let rng = match scoped_seed(
            random_seed,
            slot.descriptor_index(),
            slot.variable_name(),
            "scalar_ruin_recreate_move_selector",
        ) {
            Some(seed) => SmallRng::seed_from_u64(seed),
            None => SmallRng::from_rng(&mut rand::rng()),
        };
        Self { rng }
    }

    fn rng_mut(&mut self) -> &mut SmallRng {
        &mut self.rng
    }
}

impl fmt::Debug for ScalarRuinStreamState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ScalarRuinStreamState(..)")
    }
}

/// State stored by the future generic composed selector tree, never in a
/// selector through interior mutability.
pub(crate) enum ScalarNeighborhoodStreamState {
    Stateless,
    Ruin(ScalarRuinStreamState),
}

impl fmt::Debug for ScalarNeighborhoodStreamState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stateless => formatter.write_str("ScalarNeighborhoodStreamState::Stateless"),
            Self::Ruin(state) => formatter
                .debug_tuple("ScalarNeighborhoodStreamState::Ruin")
                .field(state)
                .finish(),
        }
    }
}

/// Cursor-owned candidate storage and one family state machine.
pub(crate) struct RuntimeScalarNeighborhoodCursor<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    store: CandidateStore<S, RuntimeScalarMove<S>>,
    state: RuntimeScalarCursorState<S>,
}

enum RuntimeScalarCursorState<S>
where
    S: PlanningSolution,
{
    Change(ChangeCursor<S>),
    Swap(SwapCursor<S>),
    NearbyChange(NearbyChangeCursor<S>),
    NearbySwap(NearbySwapCursor<S>),
    PillarChange(PillarChangeCursor<S>),
    PillarSwap(PillarSwapCursor<S>),
    RuinRecreate(RuinRecreateCursor<S>),
}

impl<S> RuntimeScalarCursorState<S>
where
    S: PlanningSolution,
{
    fn next_recipe(&mut self) -> Option<RuntimeScalarRecipe<S>> {
        match self {
            Self::Change(cursor) => cursor.next_recipe(),
            Self::Swap(cursor) => cursor.next_recipe(),
            Self::NearbyChange(cursor) => cursor.next_recipe(),
            Self::NearbySwap(cursor) => cursor.next_recipe(),
            Self::PillarChange(cursor) => cursor.next_recipe(),
            Self::PillarSwap(cursor) => cursor.next_recipe(),
            Self::RuinRecreate(cursor) => cursor.next_recipe(),
        }
    }
}

impl<S> RuntimeScalarNeighborhoodCursor<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    fn next_move(&mut self) -> Option<RuntimeScalarMove<S>> {
        self.state.next_recipe().map(RuntimeScalarMove::from_recipe)
    }
}

impl<S> MoveCursor<S, RuntimeScalarMove<S>> for RuntimeScalarNeighborhoodCursor<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.next_move().map(|mov| self.store.push(mov))
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, RuntimeScalarMove<S>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> RuntimeScalarMove<S> {
        self.store.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<RuntimeScalarMove<S>> {
        self.next_move()
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

impl<S> Iterator for RuntimeScalarNeighborhoodCursor<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    type Item = RuntimeScalarMove<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

pub(super) fn slot_identity<S>(slot: &RuntimeScalarSlot<S>) -> u64 {
    match slot {
        RuntimeScalarSlot::Static(slot) => {
            ((slot.descriptor_index as u64) << 32) ^ slot.variable_index as u64
        }
        RuntimeScalarSlot::Dynamic(slot) => ((slot.entity.0 as u64) << 32) ^ slot.variable.0 as u64,
    }
}
