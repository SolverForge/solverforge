use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::RuntimeScalarSlot;
use crate::heuristic::selector::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor,
};
use crate::heuristic::selector::EntityReference;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope};

use super::moves::RuntimeScalarConstructionMove;
use super::{FrozenScalarConstructionSlot, ScalarOrMixedSlotOrder};
use crate::phase::construction::{
    BestFitForager, ConstructionHeuristicPhase, EntityPlacer, EntityPlacerCursor, FirstFitForager,
    Placement, StrongestFitForager, WeakestFitForager,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntityOrder {
    Canonical,
    AscendingKey,
    DescendingKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueOrder {
    Canonical,
    AscendingKey,
}

pub(super) fn solve_descriptor_placement<S, D, ProgressCb>(
    config: ConstructionHeuristicConfig,
    scalar_slots: Vec<RuntimeScalarSlot<S>>,
    slot_order: Vec<ScalarOrMixedSlotOrder>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let heuristic = config.construction_heuristic_type;
    let placer = descriptor_placer(&config, scalar_slots, slot_order);

    match heuristic {
        ConstructionHeuristicType::FirstFit
        | ConstructionHeuristicType::FirstFitDecreasing
        | ConstructionHeuristicType::AllocateEntityFromQueue
        | ConstructionHeuristicType::AllocateToValueFromQueue => {
            ConstructionHeuristicPhase::new(placer, FirstFitForager::new())
                .with_construction_obligation(config.construction_obligation)
                .solve(solver_scope)
        }
        ConstructionHeuristicType::CheapestInsertion => {
            ConstructionHeuristicPhase::new(placer, BestFitForager::new())
                .with_construction_obligation(config.construction_obligation)
                .solve(solver_scope)
        }
        ConstructionHeuristicType::WeakestFit | ConstructionHeuristicType::WeakestFitDecreasing => {
            ConstructionHeuristicPhase::new(placer, WeakestFitForager::new(runtime_strength))
                .with_construction_obligation(config.construction_obligation)
                .solve(solver_scope)
        }
        ConstructionHeuristicType::StrongestFit
        | ConstructionHeuristicType::StrongestFitDecreasing => {
            ConstructionHeuristicPhase::new(placer, StrongestFitForager::new(runtime_strength))
                .with_construction_obligation(config.construction_obligation)
                .solve(solver_scope)
        }
        ConstructionHeuristicType::ListRoundRobin
        | ConstructionHeuristicType::ListCheapestInsertion
        | ConstructionHeuristicType::ListRegretInsertion
        | ConstructionHeuristicType::ListClarkeWright
        | ConstructionHeuristicType::ListKOpt => {
            panic!("descriptor-placement scalar construction received a list heuristic")
        }
    }
    true
}

/// The descriptor-placement placer a scalar construction phase drives,
/// built from the phase's frozen slots in their frozen order.
pub(crate) fn descriptor_placer<S>(
    config: &ConstructionHeuristicConfig,
    scalar_slots: Vec<RuntimeScalarSlot<S>>,
    slot_order: Vec<ScalarOrMixedSlotOrder>,
) -> RuntimeScalarConstructionPlacer<S> {
    let slots = slot_order
        .into_iter()
        .map(|entry| match entry {
            ScalarOrMixedSlotOrder::Scalar {
                scalar_index,
                construction_slot_index,
            } => FrozenScalarConstructionSlot {
                slot: scalar_slots
                    .get(scalar_index)
                    .expect("frozen scalar construction order must reference a scalar slot")
                    .clone(),
                construction_slot_index,
            },
            ScalarOrMixedSlotOrder::List { .. } => {
                panic!("descriptor-placement scalar construction cannot contain list order entries")
            }
        })
        .collect::<Vec<_>>();
    let heuristic = config.construction_heuristic_type;
    RuntimeScalarConstructionPlacer {
        slots,
        entity_order: entity_order_for(heuristic),
        value_order: value_order_for(heuristic),
        value_candidate_limit: config.value_candidate_limit,
        live_refresh: requires_live_refresh(heuristic),
    }
}

fn runtime_strength<S>(move_: &RuntimeScalarConstructionMove<S>, solution: &S) -> i64
where
    S: PlanningSolution,
{
    move_
        .slot()
        .construction_value_order_key(solution, move_.entity_index(), move_.value())
        .expect("validated runtime scalar strength construction must provide a value order key")
}

fn entity_order_for(heuristic: ConstructionHeuristicType) -> EntityOrder {
    match heuristic {
        ConstructionHeuristicType::FirstFitDecreasing
        | ConstructionHeuristicType::WeakestFitDecreasing
        | ConstructionHeuristicType::StrongestFitDecreasing => EntityOrder::DescendingKey,
        ConstructionHeuristicType::AllocateEntityFromQueue => EntityOrder::AscendingKey,
        _ => EntityOrder::Canonical,
    }
}

fn value_order_for(heuristic: ConstructionHeuristicType) -> ValueOrder {
    match heuristic {
        ConstructionHeuristicType::AllocateToValueFromQueue => ValueOrder::AscendingKey,
        _ => ValueOrder::Canonical,
    }
}

fn requires_live_refresh(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
            | ConstructionHeuristicType::AllocateEntityFromQueue
            | ConstructionHeuristicType::AllocateToValueFromQueue
    )
}

#[derive(Clone)]
pub(crate) struct RuntimeScalarConstructionPlacer<S> {
    slots: Vec<FrozenScalarConstructionSlot<S>>,
    entity_order: EntityOrder,
    value_order: ValueOrder,
    value_candidate_limit: Option<usize>,
    live_refresh: bool,
}

impl<S> std::fmt::Debug for RuntimeScalarConstructionPlacer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeScalarConstructionPlacer")
            .field("slot_count", &self.slots.len())
            .field("entity_order", &self.entity_order)
            .field("value_order", &self.value_order)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .field("live_refresh", &self.live_refresh)
            .finish()
    }
}

impl<S> RuntimeScalarConstructionPlacer<S>
where
    S: PlanningSolution,
{
    fn ordered_entity_indices<D: Director<S>>(
        &self,
        slot: &RuntimeScalarSlot<S>,
        score_director: &D,
    ) -> Vec<usize> {
        let mut indices =
            (0..slot.entity_count(score_director.working_solution())).collect::<Vec<_>>();
        if self.entity_order == EntityOrder::Canonical {
            return indices;
        }
        indices.sort_by(|left, right| {
            let left_key = slot
                .construction_entity_order_key(score_director.working_solution(), *left)
                .expect("validated runtime scalar construction must provide an entity order key");
            let right_key = slot
                .construction_entity_order_key(score_director.working_solution(), *right)
                .expect("validated runtime scalar construction must provide an entity order key");
            match self.entity_order {
                EntityOrder::Canonical => left.cmp(right),
                EntityOrder::AscendingKey => left_key.cmp(&right_key).then(left.cmp(right)),
                EntityOrder::DescendingKey => right_key.cmp(&left_key).then(left.cmp(right)),
            }
        });
        indices
    }

    /// Refills `values` with the entity's candidate values in placement
    /// order. Ascending-key order sorts stably, so equal keys keep the order
    /// the slot offered them in.
    fn ordered_values(
        &self,
        slot: &RuntimeScalarSlot<S>,
        solution: &S,
        entity_index: usize,
        values: &mut Vec<usize>,
    ) {
        values.clear();
        slot.extend_candidate_values(solution, entity_index, self.value_candidate_limit, values);
        if self.value_order == ValueOrder::AscendingKey {
            values.sort_by_cached_key(|value| {
                slot.construction_value_order_key(solution, entity_index, *value)
                    .expect("validated runtime scalar queue construction must provide a value order key")
            });
        }
    }
}

pub(crate) struct RuntimeScalarCandidateCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, RuntimeScalarConstructionMove<S>>,
    values: Vec<usize>,
    next_value: usize,
    slot: RuntimeScalarSlot<S>,
    entity_index: usize,
}

impl<S> MoveCursor<S, RuntimeScalarConstructionMove<S>> for RuntimeScalarCandidateCursor<S>
where
    S: PlanningSolution,
{
    fn next_candidate(&mut self, _solution: &S) -> Option<CandidateId> {
        let value = *self.values.get(self.next_value)?;
        self.next_value += 1;
        Some(self.store.push(RuntimeScalarConstructionMove::new(
            self.slot.clone(),
            self.entity_index,
            value,
        )))
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, RuntimeScalarConstructionMove<S>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> RuntimeScalarConstructionMove<S> {
        self.store.take_candidate(id)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.store.release_candidate(id)
    }
}

pub(crate) struct RuntimeScalarConstructionCursor<'a, S>
where
    S: PlanningSolution,
{
    placer: &'a RuntimeScalarConstructionPlacer<S>,
    next_slot_index: usize,
    active_slot_index: Option<usize>,
    entity_indices: std::vec::IntoIter<usize>,
    /// The candidate store and value buffer of the last recycled placement,
    /// reused by the next one so placements do not reallocate them.
    spare_store: CandidateStore<S, RuntimeScalarConstructionMove<S>>,
    spare_values: Vec<usize>,
}

impl<S> EntityPlacer<S, RuntimeScalarConstructionMove<S>> for RuntimeScalarConstructionPlacer<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    type Cursor<'a>
        = RuntimeScalarConstructionCursor<'a, S>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, _score_director: &D) -> Self::Cursor<'a> {
        RuntimeScalarConstructionCursor {
            placer: self,
            next_slot_index: 0,
            active_slot_index: None,
            entity_indices: Vec::new().into_iter(),
            spare_store: CandidateStore::new(),
            spare_values: Vec::new(),
        }
    }
}

impl<S> EntityPlacerCursor<S, RuntimeScalarConstructionMove<S>>
    for RuntimeScalarConstructionCursor<'_, S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    type CandidateCursor = RuntimeScalarCandidateCursor<S>;

    fn next_placement<D, IsCompleted, ShouldStop>(
        &mut self,
        score_director: &D,
        mut is_completed: IsCompleted,
        mut should_stop: ShouldStop,
    ) -> Option<Placement<S, RuntimeScalarConstructionMove<S>, Self::CandidateCursor>>
    where
        D: Director<S>,
        IsCompleted:
            FnMut(&Placement<S, RuntimeScalarConstructionMove<S>, Self::CandidateCursor>) -> bool,
        ShouldStop: FnMut() -> bool,
    {
        if self.placer.live_refresh {
            self.next_slot_index = 0;
            self.active_slot_index = None;
            self.entity_indices = Vec::new().into_iter();
        }
        while !should_stop() {
            let Some(entity_index) = self.entity_indices.next() else {
                let slot_index = self.next_slot_index;
                let frozen = self.placer.slots.get(slot_index)?;
                self.next_slot_index += 1;
                self.active_slot_index = Some(slot_index);
                self.entity_indices = self
                    .placer
                    .ordered_entity_indices(&frozen.slot, score_director)
                    .into_iter();
                continue;
            };
            let frozen = &self.placer.slots[self
                .active_slot_index
                .expect("runtime scalar construction must retain an active slot")];
            let solution = score_director.working_solution();
            if frozen.slot.current_value(solution, entity_index).is_some() {
                continue;
            }
            self.placer.ordered_values(
                &frozen.slot,
                solution,
                entity_index,
                &mut self.spare_values,
            );
            if self.spare_values.is_empty() {
                continue;
            }
            let placement = Placement::new(
                EntityReference::new(frozen.slot.descriptor_index(), entity_index),
                RuntimeScalarCandidateCursor {
                    store: std::mem::take(&mut self.spare_store),
                    values: std::mem::take(&mut self.spare_values),
                    next_value: 0,
                    slot: frozen.slot.clone(),
                    entity_index,
                },
            )
            .with_slot_id(crate::phase::construction::ConstructionSlotId::new(
                frozen.construction_slot_index,
                entity_index,
            ))
            .with_keep_current_legal(frozen.slot.allows_unassigned());
            if !is_completed(&placement) {
                return Some(placement);
            }
            self.recycle(placement);
        }
        None
    }

    fn recycle(
        &mut self,
        placement: Placement<S, RuntimeScalarConstructionMove<S>, Self::CandidateCursor>,
    ) {
        let RuntimeScalarCandidateCursor {
            mut store, values, ..
        } = placement.into_candidates();
        store.reset();
        self.spare_store = store;
        self.spare_values = values;
    }
}
