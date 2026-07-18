use std::any::{Any, TypeId};

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
};
use solverforge_core::domain::{
    EntityClassId, EntityCollectionExtractor, EntityDescriptor, PlanningSolution,
    SolutionDescriptor, ValueRangeType, VariableDescriptor, VariableId,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::assignment_candidate::{AssignmentMoveIntent, ScalarAssignmentMoveOptions};
use super::assignment_entity::required_value_degrees;
use super::assignment_path::{assignment_move_for_entity_value, AssignmentRequest};
use super::assignment_state::ScalarAssignmentState;
use super::assignment_stream::ScalarAssignmentMoveCursor;
use super::build_scalar_group_construction;
use crate::builder::{
    bind_scalar_groups, RuntimeModel, ScalarAssignmentBinding, ScalarGroupBinding,
    ScalarGroupBindingKind, ScalarVariableSlot, ValueSource, VariableSlot,
};
use crate::descriptor::{collect_bindings, ResolvedVariableBinding};
use crate::heuristic::selector::nearby_list_change::DefaultCrossEntityDistanceMeter;
use crate::phase::control::GENERATION_POLL_INTERVAL;
use crate::phase::Phase;
use crate::planning::{ScalarGroup, ScalarGroupLimits, ScalarTarget};
use crate::scope::SolverScope;

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
fn augmenting_assignment_observes_control_during_recursive_search() {
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
    let mut polls = 0;
    let candidate = assignment_move_for_entity_value(
        &assignment,
        &solution,
        &mut state,
        AssignmentRequest::root(2, 0, limits.max_augmenting_depth.unwrap_or(3)),
        ScalarAssignmentMoveOptions::for_construction(limits),
        AssignmentMoveIntent::required(),
        &mut || {
            polls += 1;
            polls >= 5
        },
    );

    assert!(candidate.is_none());
    assert!(polls >= 5, "control must be polled inside recursion");
    assert_eq!(state.current_value(0), Some(0));
    assert_eq!(state.current_value(1), Some(1));
    assert_eq!(state.current_value(2), None);
}

#[test]
fn interrupted_required_batch_can_resume_without_partial_state() {
    let descriptor = descriptor();
    let limits = ScalarGroupLimits {
        max_augmenting_depth: Some(4),
        max_rematch_size: Some(8),
        ..ScalarGroupLimits::new()
    };
    let assignment = assignment_binding(&descriptor, limits);
    let entity_total = 100;
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![None; entity_total],
        candidates: (0..entity_total).map(|value| vec![value]).collect(),
    };
    let options = ScalarAssignmentMoveOptions::for_construction(limits);
    let mut reference = ScalarAssignmentMoveCursor::required_construction(
        assignment.clone(),
        solution.clone(),
        options,
    );
    let mut reference_polls = 0;
    let complete = reference
        .next_move_with_control(&mut || {
            reference_polls += 1;
            false
        })
        .expect("uninterrupted required batch must be generated");
    assert_eq!(complete.edits().len(), entity_total);

    let mut cursor =
        ScalarAssignmentMoveCursor::required_construction(assignment, solution, options);
    let mut polls = 0;

    let interrupted = cursor.next_move_with_control(&mut || {
        polls += 1;
        polls >= reference_polls
    });
    assert!(interrupted.is_none());

    let resumed = cursor
        .next_move_with_control(&mut || false)
        .expect("required batch must restart after interrupted generation");
    assert_eq!(resumed.edits().len(), entity_total);
}

static PREPROCESSING_CANDIDATE_READS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn tracked_preprocessing_candidates(plan: &AssignmentPlan, entity: usize, _: usize) -> &[usize] {
    let previous = PREPROCESSING_CANDIDATE_READS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        previous, 0,
        "degree preprocessing read candidates after control requested interruption"
    );
    &plan.candidates[entity]
}

#[test]
fn required_batch_polls_control_between_scarcity_and_degree_preprocessing() {
    PREPROCESSING_CANDIDATE_READS.store(0, std::sync::atomic::Ordering::SeqCst);
    let descriptor = descriptor();
    let limits = ScalarGroupLimits::new();
    let assignment = assignment_binding_with_candidate_values(
        &descriptor,
        limits,
        tracked_preprocessing_candidates,
    );
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![None],
        candidates: vec![vec![0]],
    };
    let options = ScalarAssignmentMoveOptions::for_construction(limits);
    let mut cursor =
        ScalarAssignmentMoveCursor::required_construction(assignment, solution, options);
    let interrupted = cursor.next_move_with_control(&mut || {
        PREPROCESSING_CANDIDATE_READS.load(std::sync::atomic::Ordering::SeqCst) >= 1
    });

    assert!(interrupted.is_none());
    assert_eq!(
        PREPROCESSING_CANDIDATE_READS.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[test]
fn required_value_degrees_bounds_control_polling_for_dense_rows() {
    let descriptor = descriptor();
    let limits = ScalarGroupLimits::new();
    let assignment = assignment_binding(&descriptor, limits);
    let candidate_count = GENERATION_POLL_INTERVAL * 2;
    let solution = AssignmentPlan {
        score: None,
        assignments: vec![None],
        candidates: vec![(0..candidate_count).collect()],
    };
    let mut polls = 0;

    let degrees = required_value_degrees(&assignment, &solution, &[0], None, &mut || {
        polls += 1;
        false
    })
    .expect("uninterrupted degree preprocessing must complete");

    assert_eq!(degrees.len(), candidate_count);
    assert_eq!(polls, 4, "poll once per row, interval, and final boundary");
}
