use std::collections::HashMap;

#[derive(Clone, Debug)]
struct CoverageSlot {
    required: bool,
    day: usize,
    assigned: Option<usize>,
    values: Vec<usize>,
    assignment_penalty: i64,
    worker_penalties: Vec<i64>,
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
        let uncovered_required = if self.working_solution.penalize_uncovered_required {
            self.working_solution
                .slots
                .iter()
                .filter(|slot| slot.required && slot.assigned.is_none())
                .count()
        } else {
            0
        };
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
            .filter_map(|slot| {
                let worker = slot.assigned?;
                let worker_penalty = slot.worker_penalties.get(worker).copied().unwrap_or(0);
                Some(slot.assignment_penalty + worker_penalty)
            })
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
        worker_penalties: Vec::new(),
    }
}

fn coverage_slot_with_worker_penalties(
    required: bool,
    day: usize,
    assigned: Option<usize>,
    values: &[usize],
    worker_penalties: &[i64],
) -> CoverageSlot {
    CoverageSlot {
        required,
        day,
        assigned,
        values: values.to_vec(),
        assignment_penalty: 0,
        worker_penalties: worker_penalties.to_vec(),
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

fn coverage_position_key(solution: &CoveragePlan, entity_index: usize) -> i64 {
    let slot = &solution.slots[entity_index];
    let day = match i64::try_from(slot.day) {
        Ok(day) => day,
        Err(_) => return i64::MAX,
    };
    let entity = match i64::try_from(entity_index) {
        Ok(entity) => entity,
        Err(_) => return i64::MAX,
    };
    day.saturating_mul(100).saturating_add(entity)
}

fn coverage_sequence_key(
    solution: &CoveragePlan,
    entity_index: usize,
    _worker: usize,
) -> Option<usize> {
    Some(solution.slots[entity_index].day)
}

fn coverage_entity_order(solution: &CoveragePlan, entity_index: usize) -> i64 {
    coverage_position_key(solution, entity_index)
}

fn coverage_value_order(_solution: &CoveragePlan, _entity_index: usize, value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn assignment_model() -> RuntimeModel<CoveragePlan, usize, DefaultMeter, DefaultMeter> {
    assignment_model_with_limits(ScalarGroupLimits {
        max_augmenting_depth: Some(3),
        ..ScalarGroupLimits::new()
    })
}

fn assignment_model_with_limits(
    limits: ScalarGroupLimits,
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
    RuntimeModel::new(variables).with_scalar_groups(bind_scalar_groups(
        vec![ScalarGroup::assignment(
            "slot_assignment",
            ScalarTarget::from_descriptor_index(0, "worker"),
        )
        .with_required_entity(coverage_required)
        .with_capacity_key(coverage_capacity_key)
        .with_position_key(coverage_position_key)
        .with_sequence_key(coverage_sequence_key)
        .with_entity_order(coverage_entity_order)
        .with_value_order(coverage_value_order)
        .with_limits(limits)],
        &[scalar_slot],
    ))
}

fn assignment_config() -> ConstructionHeuristicConfig {
    ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::FirstFit,
        construction_obligation: ConstructionObligation::AssignWhenCandidateExists,
        group_name: Some("slot_assignment".to_string()),
        ..ConstructionHeuristicConfig::default()
    }
}

#[test]
#[should_panic(
    expected = "construction heuristic targets assignment-owned scalar variable CoverageSlot.worker"
)]
fn explicit_scalar_construction_rejects_assignment_owned_variable() {
    let config = ConstructionHeuristicConfig {
        construction_heuristic_type: ConstructionHeuristicType::FirstFit,
        target: VariableTargetConfig {
            entity_class: Some("CoverageSlot".to_string()),
            variable_name: Some("worker".to_string()),
        },
        ..ConstructionHeuristicConfig::default()
    };
    let _ = solve_assignment_with_config_and_model(
        coverage_plan(1, vec![coverage_slot(true, 0, None, &[0])]),
        config,
        assignment_model(),
    );
}

fn solve_assignment(plan: CoveragePlan) -> SolverScope<'static, CoveragePlan, CoverageDirector> {
    solve_assignment_with_config_and_model(plan, assignment_config(), assignment_model())
}

fn solve_assignment_with_config_and_model(
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

fn solve_default_assignment(
    plan: CoveragePlan,
) -> SolverScope<'static, CoveragePlan, CoverageDirector> {
    let descriptor = coverage_plan_descriptor();
    let director = CoverageDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut phase = Construction::new(None, descriptor, assignment_model());
    phase.solve(&mut solver_scope);
    solver_scope
}

fn solve_default_assignment_with_expired_time_limit(
    plan: CoveragePlan,
) -> SolverScope<'static, CoveragePlan, CoverageDirector> {
    let descriptor = coverage_plan_descriptor();
    let director = CoverageDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    solver_scope.set_time_limit(std::time::Duration::ZERO);
    let mut phase = Construction::new(None, descriptor, assignment_model());
    phase.solve(&mut solver_scope);
    solver_scope
}

fn selector_assignment_moves_for_plan(
    plan: &CoveragePlan,
    limits: ScalarGroupLimits,
    max_moves: usize,
) -> Vec<crate::heuristic::r#move::CompoundScalarMove<CoveragePlan>> {
    let model = assignment_model_with_limits(limits);
    let group = &model.scalar_groups()[0];
    let crate::builder::ScalarGroupBindingKind::Assignment(assignment) = group.kind else {
        panic!("test model should contain an assignment-backed scalar group");
    };
    let options =
        crate::phase::construction::grouped_scalar::ScalarAssignmentMoveOptions::for_selector(
            group.limits,
            None,
            max_moves,
            0,
        );
    crate::phase::construction::grouped_scalar::selector_assignment_moves(
        &assignment,
        plan,
        options,
    )
}

#[test]
fn scalar_assignment_construction_fills_required_slots_with_free_capacity() {
    let solver_scope = solve_assignment(coverage_plan(
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
fn scalar_assignment_live_construction_generates_only_current_assignment_placement() {
    let slot_count = 12;
    let solver_scope = solve_assignment(coverage_plan(
        1,
        (0..slot_count)
            .map(|day| coverage_slot(true, day, None, &[0]))
            .collect(),
    ));

    assert!(solver_scope
        .working_solution()
        .slots
        .iter()
        .all(|slot| slot.assigned.is_some()));
    assert_eq!(solver_scope.stats().moves_generated, slot_count as u64);
}

#[test]
fn scalar_assignment_first_fit_uses_cursor_default_for_assignment_alternatives() {
    let slot_count = 3;
    let values = (0..12).collect::<Vec<_>>();
    let solver_scope = solve_assignment_with_config_and_model(
        coverage_plan(
            12,
            (0..slot_count)
                .map(|day| coverage_slot(true, day, None, &values))
                .collect(),
        ),
        assignment_config(),
        assignment_model_with_limits(ScalarGroupLimits {
            max_rematch_size: Some(3),
            ..ScalarGroupLimits::new()
        }),
    );

    assert!(solver_scope
        .working_solution()
        .slots
        .iter()
        .all(|slot| slot.assigned.is_some()));
    assert_eq!(solver_scope.stats().moves_generated, 36);
}
