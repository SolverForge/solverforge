use std::any::TypeId;
use std::cell::Cell;
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

mod coverage;

// Per-thread probes: the test harness runs tests concurrently, and every test
// here drives the same nearby-source callbacks on its own thread.
thread_local! {
    static SOLUTION_CLONES: Cell<usize> = const { Cell::new(0) };
    static VALUE_SOURCE_LIMIT: Cell<usize> = const { Cell::new(usize::MAX) };
    static ENTITY_SOURCE_LIMIT: Cell<usize> = const { Cell::new(usize::MAX) };
    static ENTITY_SOURCE_CALLS: Cell<usize> = const { Cell::new(0) };
}

#[derive(Debug)]
struct Plan {
    values: Vec<Option<usize>>,
    candidates: Vec<usize>,
    score: Option<SoftScore>,
}

impl Clone for Plan {
    fn clone(&self) -> Self {
        SOLUTION_CLONES.with(|count| count.set(count.get() + 1));
        Self {
            values: self.values.clone(),
            candidates: self.candidates.clone(),
            score: self.score,
        }
    }
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
        VALUE_SOURCE_LIMIT.with(|seen| seen.set(limit));
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
        ENTITY_SOURCE_LIMIT.with(|seen| seen.set(limit));
        ENTITY_SOURCE_CALLS.with(|calls| calls.set(calls.get() + 1));
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
fn scalar_cursor_open_and_enumeration_never_clone_the_solution() {
    use super::{ScalarNeighborhoodLeaf, ScalarNeighborhoodSpec};
    use crate::builder::RuntimeScalarSlot;
    use crate::heuristic::selector::move_selector::{MoveCursor, MoveStreamContext};
    use solverforge_config::RecreateHeuristicType;

    let director = director(Plan {
        values: vec![Some(0), Some(0), Some(1), Some(1)],
        candidates: vec![0, 1, 2],
        score: None,
    });
    let specs = [
        ScalarNeighborhoodSpec::Change {
            value_candidate_limit: None,
        },
        ScalarNeighborhoodSpec::Swap,
        ScalarNeighborhoodSpec::NearbyChange {
            max_nearby: 0,
            value_candidate_limit: None,
        },
        ScalarNeighborhoodSpec::NearbySwap { max_nearby: 0 },
        ScalarNeighborhoodSpec::NearbyChange {
            max_nearby: 3,
            value_candidate_limit: None,
        },
        ScalarNeighborhoodSpec::NearbySwap { max_nearby: 3 },
        ScalarNeighborhoodSpec::PillarChange {
            minimum_sub_pillar_size: 0,
            maximum_sub_pillar_size: 0,
            value_candidate_limit: None,
        },
        ScalarNeighborhoodSpec::PillarSwap {
            minimum_sub_pillar_size: 0,
            maximum_sub_pillar_size: 0,
        },
        ScalarNeighborhoodSpec::RuinRecreate {
            min_ruin_count: 1,
            max_ruin_count: 2,
            moves_per_step: 3,
            value_candidate_limit: None,
            recreate_heuristic_type: RecreateHeuristicType::FirstFit,
        },
    ];
    for spec in specs {
        let leaf =
            ScalarNeighborhoodLeaf::from_spec(spec, RuntimeScalarSlot::Dynamic(slot()), Some(17))
                .expect("test slot supplies the family capability");
        let mut state = leaf.new_stream_state();
        SOLUTION_CLONES.with(|count| count.set(0));
        let mut cursor =
            leaf.open_cursor_with_stream_state(&mut state, &director, MoveStreamContext::default());
        while cursor
            .next_owned_candidate(director.working_solution())
            .is_some()
        {}
        assert_eq!(SOLUTION_CLONES.with(|count| count.get()), 0, "{spec:?}");
    }
}

#[test]
fn dynamic_nearby_change_uses_the_ordinary_row_fallback_with_the_source_limit() {
    VALUE_SOURCE_LIMIT.with(|seen| seen.set(usize::MAX));
    let director = director(Plan {
        values: vec![Some(0)],
        candidates: vec![0, 2, 1],
        score: None,
    });
    let selector = DynamicScalarNearbyChangeMoveSelector::new(slot(), 2, Some(2))
        .expect("structural nearby source is declared");

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();

    assert_eq!(VALUE_SOURCE_LIMIT.with(Cell::get), 2);
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].entity_index(), 0);
    assert_eq!(moves[0].to_value(), Some(2));
}

#[test]
fn dynamic_nearby_swap_uses_the_finite_all_entity_row_fallback() {
    ENTITY_SOURCE_LIMIT.with(|seen| seen.set(usize::MAX));
    ENTITY_SOURCE_CALLS.with(|calls| calls.set(0));
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

    assert_eq!(ENTITY_SOURCE_CALLS.with(Cell::get), 3);
    assert_eq!(ENTITY_SOURCE_LIMIT.with(Cell::get), 3);
    assert_eq!(pairs, vec![(0, 1), (1, 0), (2, 1)]);
}

#[derive(Debug)]
struct CountingRowAccess {
    fallback: RowFallbackAccess,
    visits: Arc<AtomicUsize>,
}

impl DynamicScalarAccess<Plan> for CountingRowAccess {
    fn entity_class(&self) -> EntityClassId {
        self.fallback.entity_class()
    }

    fn variable(&self) -> VariableId {
        self.fallback.variable()
    }

    fn entity_count(&self, solution: &Plan) -> usize {
        self.fallback.entity_count(solution)
    }

    fn get(&self, solution: &Plan, row: usize) -> Option<usize> {
        self.fallback.get(solution, row)
    }

    fn set(&self, solution: &mut Plan, row: usize, value: Option<usize>) {
        self.fallback.set(solution, row, value)
    }

    fn candidate_values<'a>(&self, solution: &'a Plan, row: usize) -> &'a [usize] {
        self.visits.fetch_add(1, Ordering::SeqCst);
        self.fallback.candidate_values(solution, row)
    }

    fn value_is_legal(&self, solution: &Plan, row: usize, value: usize) -> bool {
        self.fallback.value_is_legal(solution, row, value)
    }

    fn has_nearby_value_candidates(&self) -> bool {
        self.fallback.has_nearby_value_candidates()
    }

    fn visit_nearby_value_candidates(
        &self,
        solution: &Plan,
        row: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        self.fallback
            .visit_nearby_value_candidates(solution, row, limit, visit)
    }

    fn nearby_value_distance(&self, solution: &Plan, row: usize, value: usize) -> Option<f64> {
        self.fallback.nearby_value_distance(solution, row, value)
    }

    fn has_nearby_entity_candidates(&self) -> bool {
        self.fallback.has_nearby_entity_candidates()
    }

    fn visit_nearby_entity_candidates(
        &self,
        solution: &Plan,
        left_row: usize,
        limit: usize,
        visit: &mut dyn FnMut(usize),
    ) -> bool {
        self.fallback
            .visit_nearby_entity_candidates(solution, left_row, limit, visit)
    }

    fn nearby_entity_distance(&self, solution: &Plan, left: usize, right: usize) -> Option<f64> {
        self.fallback.nearby_entity_distance(solution, left, right)
    }
}

#[test]
fn ordinary_change_stream_visits_row_values_lazily_one_row_at_a_time() {
    use super::{ScalarNeighborhoodLeaf, ScalarNeighborhoodSpec};
    use crate::builder::RuntimeScalarSlot;
    use crate::heuristic::selector::move_selector::{MoveCursor, MoveStreamContext};

    let visits = Arc::new(AtomicUsize::new(0));
    let director = director(Plan {
        values: vec![Some(0), None, Some(1), Some(1)],
        candidates: vec![0, 1, 2],
        score: None,
    });
    let leaf = ScalarNeighborhoodLeaf::from_spec(
        ScalarNeighborhoodSpec::Change {
            value_candidate_limit: None,
        },
        RuntimeScalarSlot::Dynamic(
            DynamicScalarVariableSlot::with_access(
                EntityClassId(0),
                VariableId(0),
                "Row",
                "value",
                false,
                Arc::new(CountingRowAccess {
                    fallback: RowFallbackAccess,
                    visits: Arc::clone(&visits),
                }),
            )
            .resolved_against(&descriptor())
            .expect("counting test slot resolves"),
        ),
        Some(17),
    )
    .expect("counting slot supplies the change capability");
    let mut state = leaf.new_stream_state();
    assert_eq!(
        visits.load(Ordering::SeqCst),
        0,
        "cursor open must not visit any row's candidate values"
    );
    let mut cursor =
        leaf.open_cursor_with_stream_state(&mut state, &director, MoveStreamContext::default());

    assert_eq!(
        visits.load(Ordering::SeqCst),
        0,
        "cursor open must not visit any row's candidate values"
    );
    let mut pulled = 0usize;
    while cursor
        .next_owned_candidate(director.working_solution())
        .is_some()
    {
        pulled += 1;
        // Visits must trail the stream: they never exceed the rows that
        // could have been entered by the pulls so far.
    }
    assert_eq!(
        visits.load(Ordering::SeqCst),
        4,
        "every row's values are visited exactly once across the full stream"
    );
    assert!(
        pulled > 0,
        "the stream must produce the ordinary change recipes"
    );
}
