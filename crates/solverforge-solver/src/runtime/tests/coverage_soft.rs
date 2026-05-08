#[derive(Clone, Debug)]
struct SoftCoveragePlan {
    score: Option<SoftScore>,
    worker_count: usize,
    slots: Vec<CoverageSlot>,
}

#[derive(Clone, Debug)]
struct SoftCoverageDirector {
    working_solution: SoftCoveragePlan,
    descriptor: SolutionDescriptor,
}

impl PlanningSolution for SoftCoveragePlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Director<SoftCoveragePlan> for SoftCoverageDirector {
    fn working_solution(&self) -> &SoftCoveragePlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut SoftCoveragePlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let assignment_penalty = self
            .working_solution
            .slots
            .iter()
            .filter(|slot| slot.assigned.is_some())
            .map(|slot| slot.assignment_penalty)
            .sum::<i64>();
        let score = SoftScore::of(-assignment_penalty);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> SoftCoveragePlan {
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

fn soft_coverage_values(
    solution: &SoftCoveragePlan,
    entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    &solution.slots[entity_index].values
}

fn soft_coverage_required(solution: &SoftCoveragePlan, entity_index: usize) -> bool {
    solution.slots[entity_index].required
}

fn soft_coverage_capacity_key(
    solution: &SoftCoveragePlan,
    entity_index: usize,
    worker: usize,
) -> Option<usize> {
    Some(solution.slots[entity_index].day * solution.worker_count + worker)
}

fn soft_coverage_entity_order(solution: &SoftCoveragePlan, entity_index: usize) -> i64 {
    (solution.slots[entity_index].day * 100 + entity_index) as i64
}

fn soft_coverage_value_order(
    _solution: &SoftCoveragePlan,
    _entity_index: usize,
    value: usize,
) -> i64 {
    value as i64
}

fn soft_coverage_model() -> RuntimeModel<SoftCoveragePlan, usize, DefaultMeter, DefaultMeter> {
    let scalar_slot = ScalarVariableSlot::new(
        0,
        0,
        "CoverageSlot",
        |solution: &SoftCoveragePlan| solution.slots.len(),
        "worker",
        |solution, entity_index, _variable_index| solution.slots[entity_index].assigned,
        |solution, entity_index, _variable_index, value| {
            solution.slots[entity_index].assigned = value;
        },
        ValueSource::EntitySlice {
            values_for_entity: soft_coverage_values,
        },
        true,
    );
    RuntimeModel::new(vec![VariableSlot::Scalar(scalar_slot)]).with_coverage_groups(
        bind_coverage_groups(
            vec![
                CoverageGroup::new(
                    "slot_coverage",
                    ScalarTarget::from_descriptor_index(0, "worker"),
                )
                .with_required_slot(soft_coverage_required)
                .with_capacity_key(soft_coverage_capacity_key)
                .with_entity_order(soft_coverage_entity_order)
                .with_value_order(soft_coverage_value_order),
            ],
            &[scalar_slot],
        ),
    )
}

fn soft_coverage_plan_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("SoftCoveragePlan", TypeId::of::<SoftCoveragePlan>()).with_entity(
        EntityDescriptor::new("CoverageSlot", TypeId::of::<CoverageSlot>(), "slots")
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "CoverageSlot",
                "slots",
                |solution: &SoftCoveragePlan| &solution.slots,
                |solution: &mut SoftCoveragePlan| &mut solution.slots,
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

fn solve_soft_coverage(
    plan: SoftCoveragePlan,
) -> SolverScope<'static, SoftCoveragePlan, SoftCoverageDirector> {
    let descriptor = soft_coverage_plan_descriptor();
    let director = SoftCoverageDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut phase = Construction::new(Some(coverage_config()), descriptor, soft_coverage_model());
    phase.solve(&mut solver_scope);
    solver_scope
}

#[test]
fn coverage_construction_forces_required_assignment_for_soft_score_worse_move() {
    let solver_scope = solve_soft_coverage(SoftCoveragePlan {
        score: None,
        worker_count: 1,
        slots: vec![coverage_slot_with_penalty(true, 0, None, &[0], 7)],
    });

    assert_eq!(solver_scope.working_solution().slots[0].assigned, Some(0));
    assert_eq!(solver_scope.current_score().copied(), Some(SoftScore::of(-7)));
    assert_eq!(solver_scope.stats().coverage_required_remaining, 0);
}
