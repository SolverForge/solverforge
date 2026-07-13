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
    let placer = RuntimeScalarConstructionPlacer {
        slots,
        entity_order: entity_order_for(heuristic),
        value_order: value_order_for(heuristic),
        value_candidate_limit: config.value_candidate_limit,
        live_refresh: requires_live_refresh(heuristic),
    };

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
struct RuntimeScalarConstructionPlacer<S> {
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

    fn ordered_values(
        &self,
        slot: &RuntimeScalarSlot<S>,
        solution: &S,
        entity_index: usize,
    ) -> Vec<usize> {
        let mut values = Vec::new();
        slot.visit_candidate_values(
            solution,
            entity_index,
            self.value_candidate_limit,
            &mut |value| values.push(value),
        );
        if self.value_order == ValueOrder::AscendingKey {
            let mut indexed = values.into_iter().enumerate().collect::<Vec<_>>();
            indexed.sort_by(|(left_order, left_value), (right_order, right_value)| {
                let left_key = slot
                    .construction_value_order_key(solution, entity_index, *left_value)
                    .expect("validated runtime scalar queue construction must provide a value order key");
                let right_key = slot
                    .construction_value_order_key(solution, entity_index, *right_value)
                    .expect("validated runtime scalar queue construction must provide a value order key");
                left_key.cmp(&right_key).then(left_order.cmp(right_order))
            });
            return indexed.into_iter().map(|(_, value)| value).collect();
        }
        values
    }
}

struct RuntimeScalarCandidateCursor<S>
where
    S: PlanningSolution,
{
    store: CandidateStore<S, RuntimeScalarConstructionMove<S>>,
    values: std::vec::IntoIter<usize>,
    slot: RuntimeScalarSlot<S>,
    entity_index: usize,
}

impl<S> MoveCursor<S, RuntimeScalarConstructionMove<S>> for RuntimeScalarCandidateCursor<S>
where
    S: PlanningSolution,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        let value = self.values.next()?;
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

struct RuntimeScalarConstructionCursor<'a, S>
where
    S: PlanningSolution,
{
    placer: &'a RuntimeScalarConstructionPlacer<S>,
    next_slot_index: usize,
    active_slot_index: Option<usize>,
    entity_indices: std::vec::IntoIter<usize>,
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
            let values = self
                .placer
                .ordered_values(&frozen.slot, solution, entity_index);
            if values.is_empty() {
                continue;
            }
            let placement = Placement::new(
                EntityReference::new(frozen.slot.descriptor_index(), entity_index),
                RuntimeScalarCandidateCursor {
                    store: CandidateStore::with_capacity(values.len()),
                    values: values.into_iter(),
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
        }
        None
    }
}
