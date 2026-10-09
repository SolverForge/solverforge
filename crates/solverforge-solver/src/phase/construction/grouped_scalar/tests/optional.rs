use super::*;
use solverforge_scoring::Director;

struct OptionalDirector {
    plan: AssignmentPlan,
    descriptor: SolutionDescriptor,
}

impl Director<AssignmentPlan> for OptionalDirector {
    fn working_solution(&self) -> &AssignmentPlan {
        &self.plan
    }
    fn working_solution_mut(&mut self) -> &mut AssignmentPlan {
        &mut self.plan
    }
    fn calculate_score(&mut self) -> SoftScore {
        let score = SoftScore::of(
            (if self.plan.assignments[0].is_some() {
                -10
            } else {
                0
            }) + (if self.plan.assignments[1].is_some() {
                1
            } else {
                0
            }),
        );
        self.plan.set_score(Some(score));
        score
    }
    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }
    fn clone_working_solution(&self) -> AssignmentPlan {
        self.plan.clone()
    }
    fn before_variable_changed(&mut self, _: usize, _: usize) {}
    fn after_variable_changed(&mut self, _: usize, _: usize) {}
    fn entity_count(&self, index: usize) -> Option<usize> {
        (index == 0).then_some(self.plan.assignments.len())
    }
    fn total_entity_count(&self) -> Option<usize> {
        Some(self.plan.assignments.len())
    }
    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

fn optional_group(
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
        .with_capacity_key(capacity_key)
        .with_limits(limits)],
        &[slot],
    );
    RuntimeModel::<
        AssignmentPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    >::new(vec![VariableSlot::Scalar(slot)])
    .with_scalar_groups(groups)
    .resolve_dynamic_descriptor_indexes(descriptor)
    .expect("optional model must resolve")
    .scalar_groups()[0]
        .clone()
}

#[test]
fn completed_optional_row_does_not_starve_later_row_at_candidate_limit() {
    assert_later_optional_row_reached(Some(2));
}

#[test]
fn completed_optional_row_does_not_end_uncapped_construction() {
    assert_later_optional_row_reached(None);
}

fn assert_later_optional_row_reached(group_candidate_limit: Option<usize>) {
    let descriptor = descriptor();
    let group = optional_group(&descriptor, ScalarGroupLimits::new(), candidates);
    let bindings = collect_bindings(&descriptor)
        .into_iter()
        .map(ResolvedVariableBinding::new)
        .collect();
    let input = AssignmentPlan {
        score: None,
        assignments: vec![None, None],
        candidates: vec![vec![0, 1], vec![2, 3]],
    };
    let director = OptionalDirector {
        plan: input,
        descriptor,
    };
    let mut scope = SolverScope::new(director);
    let config = ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        construction_obligation: ConstructionObligation::PreserveUnassigned,
        group_candidate_limit,
        ..ConstructionHeuristicConfig::default()
    };
    let mut phase = build_scalar_group_construction(Some(&config), 0, group, bindings, false);

    phase.solve(&mut scope);

    assert_eq!(
        scope.working_solution().assignments,
        vec![None, Some(2)],
        "keeping an earlier optional row must not exhaust the next placement's candidate budget"
    );
}

static COMPLETED_ROW_READS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn tracked_optional_candidates(plan: &AssignmentPlan, entity: usize, _: usize) -> &[usize] {
    if entity < plan.assignments.len() - 1 {
        COMPLETED_ROW_READS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    &plan.candidates[entity]
}

#[test]
fn completed_optional_prefix_is_not_regenerated() {
    use crate::heuristic::selector::move_selector::MoveCursor;
    use crate::phase::construction::{EntityPlacer, EntityPlacerCursor};

    COMPLETED_ROW_READS.store(0, std::sync::atomic::Ordering::SeqCst);
    let descriptor = descriptor();
    let limits = ScalarGroupLimits::new();
    let group = optional_group(&descriptor, limits, tracked_optional_candidates);
    let plan = AssignmentPlan {
        score: None,
        assignments: vec![None; 64],
        candidates: (0..64)
            .map(|row| (row * 32..(row + 1) * 32).collect())
            .collect(),
    };
    let director = ScoreDirector::simple(plan, descriptor, |plan, _| plan.assignments.len());
    let placer = super::super::placer::ScalarGroupPlacer::new(
        0,
        group,
        Vec::new(),
        limits,
        ConstructionHeuristicType::CheapestInsertion,
        ConstructionObligation::PreserveUnassigned,
        false,
    );
    let mut cursor = placer.open_cursor(&director);
    let mut placement = cursor
        .next_placement(
            &director,
            |placement| {
                placement
                    .construction_target()
                    .scalar_slots()
                    .iter()
                    .any(|slot| slot.entity_index() < 63)
            },
            || false,
        )
        .expect("unfinished final row must be reached");
    let id = placement
        .candidates_mut()
        .next_candidate()
        .expect("final row must have a candidate");
    let candidate = placement
        .candidates()
        .candidate(id)
        .expect("candidate must remain borrowable");
    let crate::heuristic::selector::move_selector::MoveCandidateRef::Borrowed(mov) = candidate
    else {
        panic!("grouped assignment must retain a borrowed candidate");
    };
    assert_eq!(mov.edits()[0].entity_index, 63);
    assert_eq!(
        COMPLETED_ROW_READS.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "completed roots must be excluded before reading domains or generating moves"
    );
}
