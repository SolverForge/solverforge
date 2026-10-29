use crate::builder::context::scalar::group::bind_scalar_groups;
use crate::planning::scalar::target::ScalarTarget;
use std::any::{Any, TypeId};

#[path = "tests/augmenting_overlay.rs"]
mod augmenting_overlay;
#[path = "tests/ejection_differential.rs"]
mod ejection_differential;
#[path = "tests/neighborhood_contracts.rs"]
mod neighborhood_contracts;
#[path = "tests/optional.rs"]
mod optional;
#[path = "tests/overlay_differential.rs"]
mod overlay_differential;
#[path = "tests/placement_streaming.rs"]
mod placement_streaming;
#[path = "tests/pool_differential.rs"]
mod pool_differential;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation, SelectionOrder,
};
use solverforge_core::domain::{
    EntityClassId, EntityCollectionExtractor, EntityDescriptor, PlanningSolution,
    SolutionDescriptor, ValueRangeType, VariableDescriptor, VariableId,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::assignment_candidate::{
    order_candidates, AssignmentMoveIntent, ScalarAssignmentMoveOptions,
};
use super::assignment_entity::required_value_degrees;
use super::assignment_path::{assignment_move_for_entity_value, AssignmentRequest};
use super::assignment_state::ScalarAssignmentState;
use super::assignment_stream::ScalarAssignmentMoveCursor;
use super::build_scalar_group_construction;
use crate::builder::{
    RuntimeModel, ScalarAssignmentBinding, ScalarGroupBinding, ScalarGroupBindingKind,
    ScalarVariableSlot, ValueSource, VariableSlot,
};
use crate::descriptor::{collect_bindings, ResolvedVariableBinding};
use crate::heuristic::selector::move_selector::MoveStreamContext;
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::phase::control::GENERATION_POLL_INTERVAL;
use crate::phase::Phase;
use crate::planning::{ScalarGroup, ScalarGroupLimits};
use crate::scope::SolverScope;

mod control;
mod selector_order;

#[derive(Clone, Debug)]
struct AssignmentPlan {
    score: Option<SoftScore>,
    assignments: Vec<Option<usize>>,
    candidates: Vec<Vec<usize>>,
}

impl PlanningSolution for AssignmentPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("AssignmentPlan", TypeId::of::<AssignmentPlan>()).with_entity(
        EntityDescriptor::new("Task", TypeId::of::<Option<usize>>(), "tasks")
            .with_logical_id(EntityClassId(0))
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "Task",
                "tasks",
                |plan: &AssignmentPlan| &plan.assignments,
                |plan: &mut AssignmentPlan| &mut plan.assignments,
            )))
            .with_variable(
                VariableDescriptor::genuine("worker")
                    .with_logical_id(VariableId(0))
                    .with_allows_unassigned(true)
                    .with_value_range_type(ValueRangeType::EntityDependent)
                    .with_usize_accessors(assignment_getter, assignment_setter),
            ),
    )
}

fn assignment_getter(entity: &dyn Any) -> Option<usize> {
    *entity
        .downcast_ref::<Option<usize>>()
        .expect("Task entity must be an optional worker")
}

fn assignment_setter(entity: &mut dyn Any, value: Option<usize>) {
    *entity
        .downcast_mut::<Option<usize>>()
        .expect("Task entity must be an optional worker") = value;
}

fn entity_count(plan: &AssignmentPlan) -> usize {
    plan.assignments.len()
}

fn current_value(plan: &AssignmentPlan, entity: usize, _: usize) -> Option<usize> {
    plan.assignments[entity]
}

fn set_value(plan: &mut AssignmentPlan, entity: usize, _: usize, value: Option<usize>) {
    plan.assignments[entity] = value;
}

fn candidates(plan: &AssignmentPlan, entity: usize, _: usize) -> &[usize] {
    &plan.candidates[entity]
}

fn required(_: &AssignmentPlan, _: usize) -> bool {
    true
}

fn capacity_key(_: &AssignmentPlan, _: usize, value: usize) -> Option<usize> {
    Some(value)
}

type TestCandidateValues = for<'a> fn(&'a AssignmentPlan, usize, usize) -> &'a [usize];

fn assignment_group(
    descriptor: &SolutionDescriptor,
    limits: ScalarGroupLimits,
) -> ScalarGroupBinding<AssignmentPlan> {
    assignment_group_with_candidate_values(descriptor, limits, candidates)
}

fn assignment_group_with_candidate_values(
    descriptor: &SolutionDescriptor,
    limits: ScalarGroupLimits,
    candidate_values: TestCandidateValues,
) -> ScalarGroupBinding<AssignmentPlan> {
    let slot = ScalarVariableSlot::new(
        0,
        0,
        "Task",
        entity_count,
        "worker",
        current_value,
        set_value,
        ValueSource::EntitySlice {
            values_for_entity: candidate_values,
        },
        true,
    )
    .with_candidate_values(candidate_values);
    let groups = bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "worker_assignment",
            ScalarTarget::from_descriptor_index(0, "worker"),
        )
        .with_required_entity(required)
        .with_capacity_key(capacity_key)
        .with_limits(limits)],
        &[slot],
    );
    let model = RuntimeModel::<
        AssignmentPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    >::new(vec![VariableSlot::Scalar(slot)])
    .with_scalar_groups(groups)
    .resolve_dynamic_descriptor_indexes(descriptor)
    .expect("assignment model must resolve against its descriptor");
    model.scalar_groups()[0].clone()
}

fn assignment_binding(
    descriptor: &SolutionDescriptor,
    limits: ScalarGroupLimits,
) -> ScalarAssignmentBinding<AssignmentPlan> {
    assignment_binding_with_candidate_values(descriptor, limits, candidates)
}

fn assignment_binding_with_candidate_values(
    descriptor: &SolutionDescriptor,
    limits: ScalarGroupLimits,
    candidate_values: TestCandidateValues,
) -> ScalarAssignmentBinding<AssignmentPlan> {
    let group = assignment_group_with_candidate_values(descriptor, limits, candidate_values);
    match group.kind {
        ScalarGroupBindingKind::Assignment(assignment) => assignment,
        ScalarGroupBindingKind::Candidates { .. } => {
            panic!("test assignment group must bind assignment metadata")
        }
    }
}

static DIRECT_BATCH_CANDIDATE_READS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn tracked_direct_batch_candidates(plan: &AssignmentPlan, entity: usize, _: usize) -> &[usize] {
    DIRECT_BATCH_CANDIDATE_READS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    &plan.candidates[entity]
}

#[test]
fn required_batch_defers_augmenting_rematches() {
    DIRECT_BATCH_CANDIDATE_READS.store(0, std::sync::atomic::Ordering::SeqCst);
    let descriptor = descriptor();
    let limits = ScalarGroupLimits {
        max_augmenting_depth: Some(4),
        max_rematch_size: Some(8),
        ..ScalarGroupLimits::new()
    };
    let assignment = assignment_binding_with_candidate_values(
        &descriptor,
        limits,
        tracked_direct_batch_candidates,
    );
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![None, None, None],
        candidates: vec![vec![0, 2], vec![1, 2], vec![0, 1]],
    };
    let mut cursor = ScalarAssignmentMoveCursor::required_construction(
        &assignment,
        &solution,
        ScalarAssignmentMoveOptions::for_construction(limits),
    );

    let direct_batch = cursor
        .next_move_with_control(&assignment, &solution, &mut || false)
        .expect("direct required assignments must form a batch");

    assert_eq!(direct_batch.edits().len(), 2);
    assert_eq!(
        DIRECT_BATCH_CANDIDATE_READS.load(std::sync::atomic::Ordering::SeqCst),
        21,
        "the direct batch must not traverse augmenting rematch candidates"
    );
}

#[test]
fn required_construction_can_rematch_rows_assigned_by_its_batch_move() {
    let descriptor = descriptor();
    let group = assignment_group(
        &descriptor,
        ScalarGroupLimits {
            max_augmenting_depth: Some(4),
            max_rematch_size: Some(8),
            ..ScalarGroupLimits::new()
        },
    );
    let bindings = collect_bindings(&descriptor)
        .into_iter()
        .map(ResolvedVariableBinding::new)
        .collect();
    let input = AssignmentPlan {
        score: None,
        assignments: vec![None, None, None],
        candidates: vec![vec![0, 2], vec![1, 2], vec![0, 1]],
    };
    let director = ScoreDirector::simple(input, descriptor, |plan, _| plan.assignments.len());
    let mut scope = SolverScope::new(director);
    let config = ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
        ..ConstructionHeuristicConfig::default()
    };
    let mut phase = build_scalar_group_construction(Some(&config), 0, group, bindings, true);

    phase.solve(&mut scope);

    let assignments = &scope.working_solution().assignments;
    assert!(
        assignments.iter().all(Option::is_some),
        "required construction left assignments incomplete: {assignments:?}"
    );
    let mut assigned_values = assignments.iter().copied().flatten().collect::<Vec<_>>();
    assigned_values.sort_unstable();
    assert_eq!(assigned_values, vec![0, 1, 2]);
}

#[test]
fn required_construction_sees_global_dependent_domains_after_commits() {
    let descriptor = descriptor();
    let limits = ScalarGroupLimits {
        max_augmenting_depth: Some(4),
        max_rematch_size: Some(8),
        ..ScalarGroupLimits::new()
    };

    // Domain of row 2 reads the LIVE whole solution: it is empty until row 0
    // holds a committed assignment. No stale enumeration may close row 2.
    let global_dependent_value = |plan: &AssignmentPlan, entity: usize, _: usize| -> &[usize] {
        match entity {
            0 => &const { [0usize] },
            1 => &const { [1usize] },
            2 => {
                if plan.assignments[0].is_some() {
                    &const { [2usize] }
                } else {
                    &const { [] }
                }
            }
            _ => &const { [] },
        }
    };
    let group = assignment_group_with_candidate_values(&descriptor, limits, global_dependent_value);
    let bindings = collect_bindings(&descriptor)
        .into_iter()
        .map(ResolvedVariableBinding::new)
        .collect();
    let input = AssignmentPlan {
        score: None,
        assignments: vec![None, None, None],
        candidates: vec![vec![0], vec![1], vec![2]],
    };
    let director = ScoreDirector::simple(input, descriptor, |plan, _| plan.assignments.len());
    let mut scope = SolverScope::new(director);
    let config = ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::FirstFit,
        construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
        ..ConstructionHeuristicConfig::default()
    };
    let mut phase = build_scalar_group_construction(Some(&config), 0, group, bindings, true);
    phase.solve(&mut scope);

    assert_eq!(
        scope.working_solution().assignments,
        vec![Some(0), Some(1), Some(2)],
        "row 2 fills only after row 0 commits; stale enumeration would leave it unassigned"
    );
}

#[test]
fn required_construction_does_not_displace_a_pinned_assignment() {
    let mut descriptor = descriptor();
    descriptor.entity_descriptors[0] = descriptor.entity_descriptors[0]
        .clone()
        .with_pin_predicate(|entity| *entity.downcast_ref::<Option<usize>>().unwrap() == Some(0));
    let limits = ScalarGroupLimits {
        max_augmenting_depth: Some(4),
        max_rematch_size: Some(8),
        ..ScalarGroupLimits::new()
    };
    let group = assignment_group(&descriptor, limits);
    let bindings = collect_bindings(&descriptor)
        .into_iter()
        .map(ResolvedVariableBinding::new)
        .collect();
    let input = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), None, None],
        candidates: vec![vec![0, 1], vec![0], vec![2]],
    };
    let director = ScoreDirector::simple(input, descriptor, |plan, _| plan.assignments.len());
    let mut scope = SolverScope::new(director);
    let config = ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::FirstFit,
        construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
        ..ConstructionHeuristicConfig::default()
    };
    let mut phase = build_scalar_group_construction(Some(&config), 0, group, bindings, true);

    phase.solve(&mut scope);

    assert_eq!(scope.working_solution().assignments[0], Some(0));
    assert_eq!(scope.working_solution().assignments[1], None);
    assert_eq!(scope.working_solution().assignments[2], Some(2));
}

#[test]
fn augmenting_assignment_restores_index_after_final_feasibility() {
    let descriptor = descriptor();
    let limits = ScalarGroupLimits {
        max_augmenting_depth: Some(4),
        max_rematch_size: Some(8),
        ..ScalarGroupLimits::new()
    };
    let assignment = assignment_binding(&descriptor, limits);
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![Some(0), Some(1), None],
        candidates: vec![vec![0, 2], vec![1, 2], vec![0, 1]],
    };
    let mut state = ScalarAssignmentState::new(&assignment, &solution);
    let before = state.index_snapshot();
    let candidate = assignment_move_for_entity_value(
        &assignment,
        &solution,
        &mut state,
        AssignmentRequest::root(2, 0, limits.max_augmenting_depth.unwrap_or(3)),
        ScalarAssignmentMoveOptions::for_construction(limits),
        AssignmentMoveIntent::required(),
        &mut || false,
    )
    .expect("displacement candidate");

    // The trial edits were retracted after final feasibility: the base index
    // must equal the rebuilt reference and the next discovery must not inherit
    // speculative rows.
    assert_eq!(state.index_snapshot(), before);
    let rebuilt = ScalarAssignmentState::new(&assignment, &solution);
    assert_eq!(state.index_snapshot(), rebuilt.index_snapshot());
    let displaced = assignment_move_for_entity_value(
        &assignment,
        &solution,
        &mut state,
        AssignmentRequest::root(2, 0, limits.max_augmenting_depth.unwrap_or(3)),
        ScalarAssignmentMoveOptions::for_construction(limits),
        AssignmentMoveIntent::required(),
        &mut || false,
    );
    assert_eq!(
        displaced.map(|mov| mov.edits().len()),
        Some(candidate.edits().len())
    );
}
