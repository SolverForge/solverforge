use std::collections::HashMap;

#[derive(Clone, Debug)]
struct CoverageSlot {
    required: bool,
    day: usize,
    assigned: Option<usize>,
    values: Vec<usize>,
    assignment_penalty: i64,
}

#[derive(Clone, Debug)]
struct CoveragePlan {
    score: Option<HardSoftScore>,
    worker_count: usize,
    penalize_uncovered_required: bool,
    slots: Vec<CoverageSlot>,
}

#[derive(Clone, Debug)]
struct CoverageDirector {
    working_solution: CoveragePlan,
    descriptor: SolutionDescriptor,
}

impl PlanningSolution for CoveragePlan {
    type Score = HardSoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Director<CoveragePlan> for CoverageDirector {
    fn working_solution(&self) -> &CoveragePlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut CoveragePlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> HardSoftScore {
        let uncovered_required = self
            .working_solution
            .penalize_uncovered_required
            .then(|| {
                self.working_solution
                    .slots
                    .iter()
                    .filter(|slot| slot.required && slot.assigned.is_none())
                    .count()
            })
            .unwrap_or(0);
        let uncovered_optional = self
            .working_solution
            .slots
            .iter()
            .filter(|slot| !slot.required && slot.assigned.is_none())
            .count();
        let assignment_penalty = self
            .working_solution
            .slots
            .iter()
            .filter(|slot| slot.assigned.is_some())
            .map(|slot| slot.assignment_penalty)
            .sum::<i64>();
        let mut occupancy = HashMap::new();
        for slot in &self.working_solution.slots {
            if let Some(worker) = slot.assigned {
                *occupancy.entry((slot.day, worker)).or_insert(0usize) += 1;
            }
        }
        let capacity_conflicts = occupancy
            .values()
            .map(|count| count.saturating_sub(1))
            .sum::<usize>();
        let hard = uncovered_required + capacity_conflicts;
        let score = HardSoftScore::of(
            -(hard as i64),
            -(uncovered_optional as i64) - assignment_penalty,
        );
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> CoveragePlan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.slots.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.slots.len())
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

fn coverage_slot(
    required: bool,
    day: usize,
    assigned: Option<usize>,
    values: &[usize],
) -> CoverageSlot {
    coverage_slot_with_penalty(required, day, assigned, values, 0)
}

fn coverage_slot_with_penalty(
    required: bool,
    day: usize,
    assigned: Option<usize>,
    values: &[usize],
    assignment_penalty: i64,
) -> CoverageSlot {
    CoverageSlot {
        required,
        day,
        assigned,
        values: values.to_vec(),
        assignment_penalty,
    }
}

fn coverage_plan(worker_count: usize, slots: Vec<CoverageSlot>) -> CoveragePlan {
    CoveragePlan {
        score: None,
        worker_count,
        penalize_uncovered_required: true,
        slots,
    }
}

fn soft_preferred_coverage_plan(worker_count: usize, slots: Vec<CoverageSlot>) -> CoveragePlan {
    CoveragePlan {
        score: None,
        worker_count,
        penalize_uncovered_required: false,
        slots,
    }
}

fn coverage_plan_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("CoveragePlan", TypeId::of::<CoveragePlan>()).with_entity(
        EntityDescriptor::new("CoverageSlot", TypeId::of::<CoverageSlot>(), "slots")
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "CoverageSlot",
                "slots",
                |solution: &CoveragePlan| &solution.slots,
                |solution: &mut CoveragePlan| &mut solution.slots,
            )))
            .with_variable(
                VariableDescriptor::genuine("worker")
                    .with_allows_unassigned(true)
                    .with_value_range_type(
                        solverforge_core::domain::ValueRangeType::EntityDependent,
                    )
                    .with_usize_accessors(coverage_get_worker, coverage_set_worker),
            ),
    )
}

fn coverage_get_worker(entity: &dyn std::any::Any) -> Option<usize> {
    entity
        .downcast_ref::<CoverageSlot>()
        .expect("coverage slot expected")
        .assigned
}

fn coverage_set_worker(entity: &mut dyn std::any::Any, value: Option<usize>) {
    entity
        .downcast_mut::<CoverageSlot>()
        .expect("coverage slot expected")
        .assigned = value;
}

fn coverage_values(
    solution: &CoveragePlan,
    entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    &solution.slots[entity_index].values
}

fn coverage_required(solution: &CoveragePlan, entity_index: usize) -> bool {
    solution.slots[entity_index].required
}

fn coverage_capacity_key(
    solution: &CoveragePlan,
    entity_index: usize,
    worker: usize,
) -> Option<usize> {
    Some(solution.slots[entity_index].day * solution.worker_count + worker)
}

fn coverage_entity_order(solution: &CoveragePlan, entity_index: usize) -> i64 {
    (solution.slots[entity_index].day * 100 + entity_index) as i64
}

fn coverage_value_order(_solution: &CoveragePlan, _entity_index: usize, value: usize) -> i64 {
    value as i64
}

fn coverage_model() -> RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter> {
    coverage_model_with_limits(CoverageGroupLimits {
        max_augmenting_depth: Some(3),
        ..CoverageGroupLimits::new()
    })
}

fn coverage_model_with_limits(
    limits: CoverageGroupLimits,
) -> RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter> {
    let scalar_slot = ScalarVariableSlot::new(
        0,
        0,
        "CoverageSlot",
        |solution: &CoveragePlan| solution.slots.len(),
        "worker",
        |solution, entity_index, _variable_index| solution.slots[entity_index].assigned,
        |solution, entity_index, _variable_index, value| {
            solution.slots[entity_index].assigned = value;
        },
        ValueSource::EntitySlice {
            values_for_entity: coverage_values,
        },
        true,
    );
    let variables = vec![VariableSlot::Scalar(scalar_slot)];
    RuntimeModel::new(variables).with_coverage_groups(bind_coverage_groups(
        vec![CoverageGroup::new(
            "slot_coverage",
            ScalarTarget::from_descriptor_index(0, "worker"),
        )
        .with_required_slot(coverage_required)
        .with_capacity_key(coverage_capacity_key)
        .with_entity_order(coverage_entity_order)
        .with_value_order(coverage_value_order)
        .with_limits(limits)],
        &[scalar_slot],
    ))
}

fn coverage_config() -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::CoverageFirstFit,
        construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
        group_name: Some("slot_coverage".to_string()),
        ..ConstructionHeuristicConfig::default()
    }
}

fn solve_coverage(plan: CoveragePlan) -> SolverScope<'static, CoveragePlan, CoverageDirector> {
    solve_coverage_with_config_and_model(plan, coverage_config(), coverage_model())
}

fn solve_coverage_with_config_and_model(
    plan: CoveragePlan,
    config: ConstructionHeuristicConfig,
    model: RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter>,
) -> SolverScope<'static, CoveragePlan, CoverageDirector> {
    let descriptor = coverage_plan_descriptor();
    let director = CoverageDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut phase = Construction::new(Some(config), descriptor, model);
    phase.solve(&mut solver_scope);
    solver_scope
}

#[test]
fn coverage_construction_fills_required_slots_with_free_capacity() {
    let solver_scope = solve_coverage(coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0, 1]),
            coverage_slot(true, 0, None, &[0, 1]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert!(slots.iter().all(|slot| slot.assigned.is_some()));
    assert_ne!(slots[0].assigned, slots[1].assigned);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
}

#[test]
fn coverage_construction_ignores_repair_move_cap() {
    let model = coverage_model_with_limits(CoverageGroupLimits {
        group_candidate_limit: Some(2),
        max_moves_per_step: Some(1),
        max_augmenting_depth: Some(3),
        ..CoverageGroupLimits::new()
    });
    let plan = coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0, 1]),
            coverage_slot(true, 0, None, &[0, 1]),
        ],
    );
    let options = crate::phase::construction::coverage::CoverageMoveOptions::for_construction(
        &model.coverage_groups()[0],
        None,
        None,
    );
    let moves = crate::phase::construction::coverage::required_coverage_moves(
        &model.coverage_groups()[0],
        &plan,
        options,
    );

    assert_eq!(moves.len(), 2);
}

#[test]
fn coverage_construction_assigns_optional_only_after_required_complete() {
    let solver_scope = solve_coverage(coverage_plan(
        1,
        vec![
            coverage_slot(false, 0, None, &[0]),
            coverage_slot(true, 0, None, &[0]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, None);
    assert_eq!(slots[1].assigned, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -1))
    );
}

#[test]
fn coverage_construction_displaces_optional_occupant_for_required_slot() {
    let solver_scope = solve_coverage(coverage_plan(
        1,
        vec![
            coverage_slot(false, 0, Some(0), &[0]),
            coverage_slot(true, 0, None, &[0]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, None);
    assert_eq!(slots[1].assigned, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -1))
    );
}

#[test]
fn coverage_construction_moves_required_blocker_through_augmenting_path() {
    let solver_scope = solve_coverage(coverage_plan(
        2,
        vec![
            coverage_slot(true, 0, None, &[0]),
            coverage_slot(true, 0, Some(0), &[0, 1]),
        ],
    ));

    let slots = &solver_scope.working_solution().slots;
    assert_eq!(slots[0].assigned, Some(0));
    assert_eq!(slots[1].assigned, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
}

#[test]
fn coverage_construction_forces_required_assignment_when_hard_neutral_soft_worse() {
    let solver_scope = solve_coverage(soft_preferred_coverage_plan(
        1,
        vec![coverage_slot_with_penalty(true, 0, None, &[0], 5)],
    ));

    assert_eq!(solver_scope.working_solution().slots[0].assigned, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -5))
    );
    assert_eq!(solver_scope.stats().coverage_required_remaining, 0);
}

#[test]
fn coverage_construction_reports_remaining_required_slots_without_panic() {
    let solver_scope = solve_coverage(coverage_plan(1, vec![coverage_slot(true, 0, None, &[])]));

    assert_eq!(solver_scope.working_solution().slots[0].assigned, None);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(-1, 0))
    );
    assert_eq!(solver_scope.stats().coverage_required_remaining, 1);
}
