use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_core::domain::{
    DynamicScalarAccess, DynamicScalarVariableSlot, EntityClassId, EntityCollectionExtractor,
    EntityDescriptor, PlanningSolution, SolutionDescriptor, ValueRangeType, VariableDescriptor,
    VariableId,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::selector::{
    DynamicScalarNearbyChangeMoveSelector, DynamicScalarNearbySwapMoveSelector, MoveSelector,
};

static VALUE_SOURCE_LIMIT: AtomicUsize = AtomicUsize::new(usize::MAX);
static ENTITY_SOURCE_LIMIT: AtomicUsize = AtomicUsize::new(usize::MAX);
static ENTITY_SOURCE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
struct Plan {
    values: Vec<Option<usize>>,
    candidates: Vec<usize>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Debug)]
struct RowFallbackAccess;

impl DynamicScalarAccess<Plan> for RowFallbackAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, solution: &Plan) -> usize {
        solution.values.len()
    }

    fn get(&self, solution: &Plan, row: usize) -> Option<usize> {
        solution.values[row]
    }

    fn set(&self, solution: &mut Plan, row: usize, value: Option<usize>) {
        solution.values[row] = value;
    }

    fn candidate_values<'a>(&self, solution: &'a Plan, _row: usize) -> &'a [usize] {
        &solution.candidates
    }

    fn value_is_legal(&self, solution: &Plan, _row: usize, value: usize) -> bool {
        solution.candidates.contains(&value)
    }

    fn has_nearby_value_candidates(&self) -> bool {
        true
    }

    fn visit_nearby_value_candidates(
        &self,
        _solution: &Plan,
        _row: usize,
        limit: usize,
        _visit: &mut dyn FnMut(usize),
    ) -> bool {
        VALUE_SOURCE_LIMIT.store(limit, Ordering::SeqCst);
        false
    }

    fn nearby_value_distance(&self, _solution: &Plan, _row: usize, value: usize) -> Option<f64> {
        Some(value as f64)
    }

    fn has_nearby_entity_candidates(&self) -> bool {
        true
    }

    fn visit_nearby_entity_candidates(
        &self,
        _solution: &Plan,
        _left_row: usize,
        limit: usize,
        _visit: &mut dyn FnMut(usize),
    ) -> bool {
        ENTITY_SOURCE_LIMIT.store(limit, Ordering::SeqCst);
        ENTITY_SOURCE_CALLS.fetch_add(1, Ordering::SeqCst);
        false
    }

    fn nearby_entity_distance(
        &self,
        _solution: &Plan,
        left_row: usize,
        right_row: usize,
    ) -> Option<f64> {
        Some(left_row.abs_diff(right_row) as f64)
    }
}

fn descriptor_get(entity: &dyn std::any::Any) -> Option<usize> {
    *entity
        .downcast_ref::<Option<usize>>()
        .expect("test entity is an optional value")
}

fn descriptor_set(entity: &mut dyn std::any::Any, value: Option<usize>) {
    *entity
        .downcast_mut::<Option<usize>>()
        .expect("test entity is an optional value") = value;
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Row", TypeId::of::<Option<usize>>(), "values")
            .with_logical_id(EntityClassId(0))
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "Row",
                "values",
                |plan: &Plan| &plan.values,
                |plan: &mut Plan| &mut plan.values,
            )))
            .with_variable(
                VariableDescriptor::genuine("value")
                    .with_logical_id(VariableId(0))
                    .with_value_range_type(ValueRangeType::EntityDependent)
                    .with_usize_accessors(descriptor_get, descriptor_set),
            ),
    )
}

fn slot() -> DynamicScalarVariableSlot<Plan> {
    DynamicScalarVariableSlot::with_access(
        EntityClassId(0),
        VariableId(0),
        "Row",
        "value",
        false,
        Arc::new(RowFallbackAccess),
    )
    .resolved_against(&descriptor())
    .expect("dynamic test slot resolves")
}

fn director(plan: Plan) -> ScoreDirector<Plan, ()> {
    ScoreDirector::simple(plan, descriptor(), |plan, _| plan.values.len())
}

#[test]
fn dynamic_nearby_change_uses_the_ordinary_row_fallback_with_the_source_limit() {
    VALUE_SOURCE_LIMIT.store(usize::MAX, Ordering::SeqCst);
    let director = director(Plan {
        values: vec![Some(0)],
        candidates: vec![0, 2, 1],
        score: None,
    });
    let selector = DynamicScalarNearbyChangeMoveSelector::new(slot(), 2, Some(2))
        .expect("structural nearby source is declared");

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();

    assert_eq!(VALUE_SOURCE_LIMIT.load(Ordering::SeqCst), 2);
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].entity_index(), 0);
    assert_eq!(moves[0].to_value(), Some(2));
}

#[test]
fn dynamic_nearby_swap_uses_the_finite_all_entity_row_fallback() {
    ENTITY_SOURCE_LIMIT.store(usize::MAX, Ordering::SeqCst);
    ENTITY_SOURCE_CALLS.store(0, Ordering::SeqCst);
    let director = director(Plan {
        values: vec![Some(0), Some(1), Some(2)],
        candidates: vec![0, 1, 2],
        score: None,
    });
    let selector = DynamicScalarNearbySwapMoveSelector::new(slot(), 1)
        .expect("structural nearby source is declared");

    let pairs = selector
        .iter_moves(&director)
        .map(|mov| (mov.left_entity_index(), mov.right_entity_index()))
        .collect::<Vec<_>>();

    assert_eq!(ENTITY_SOURCE_CALLS.load(Ordering::SeqCst), 3);
    assert_eq!(ENTITY_SOURCE_LIMIT.load(Ordering::SeqCst), 3);
    assert_eq!(pairs, vec![(0, 1), (1, 0), (2, 1)]);
}
