use solverforge_core::domain::{
    DynamicListVariableSlot, DynamicModelBackend, DynamicScalarVariableSlot, EntityClassId,
    VariableId,
};

#[derive(Clone, Debug)]
struct DynamicRow;

#[derive(Clone, Debug)]
struct DynamicPlan {
    score: Option<SoftScore>,
    scalar_values: Vec<Option<usize>>,
    scalar_candidates: Vec<Vec<usize>>,
    lists: Vec<Vec<usize>>,
    list_element_count: usize,
}

impl PlanningSolution for DynamicPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    fn is_initialized(&self) -> bool {
        self.scalar_values.iter().all(Option::is_some)
            && self
                .lists
                .iter()
                .map(Vec::len)
                .sum::<usize>()
                == self.list_element_count
    }
}

impl DynamicModelBackend for DynamicPlan {
    type Score = SoftScore;

    fn entity_count(&self, entity: EntityClassId) -> usize {
        match entity.0 {
            0 => self.scalar_values.len(),
            1 => self.lists.len(),
            _ => 0,
        }
    }

    fn get_scalar(
        &self,
        _entity: EntityClassId,
        row: usize,
        _variable: VariableId,
    ) -> Option<usize> {
        self.scalar_values.get(row).copied().flatten()
    }

    fn set_scalar(
        &mut self,
        _entity: EntityClassId,
        row: usize,
        _variable: VariableId,
        value: Option<usize>,
    ) {
        if let Some(slot) = self.scalar_values.get_mut(row) {
            *slot = value;
        }
    }

    fn list_len(&self, _entity: EntityClassId, row: usize, _variable: VariableId) -> usize {
        self.lists.get(row).map(Vec::len).unwrap_or(0)
    }

    fn list_get(
        &self,
        _entity: EntityClassId,
        row: usize,
        _variable: VariableId,
        pos: usize,
    ) -> Option<usize> {
        self.lists.get(row)?.get(pos).copied()
    }

    fn list_insert(
        &mut self,
        _entity: EntityClassId,
        row: usize,
        _variable: VariableId,
        pos: usize,
        value: usize,
    ) {
        let Some(list) = self.lists.get_mut(row) else {
            return;
        };
        list.insert(pos.min(list.len()), value);
    }

    fn list_remove(
        &mut self,
        _entity: EntityClassId,
        row: usize,
        _variable: VariableId,
        pos: usize,
    ) -> Option<usize> {
        let list = self.lists.get_mut(row)?;
        (pos < list.len()).then(|| list.remove(pos))
    }

    fn candidate_values(
        &self,
        _entity: EntityClassId,
        row: usize,
        _variable: VariableId,
    ) -> &[usize] {
        self.scalar_candidates
            .get(row)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn list_element_count(&self, _entity: EntityClassId, _variable: VariableId) -> usize {
        self.list_element_count
    }

    fn list_assigned_elements(&self, _entity: EntityClassId, _variable: VariableId) -> Vec<usize> {
        self.lists
            .iter()
            .flat_map(|list| list.iter().copied())
            .collect()
    }
}

fn dynamic_descriptor() -> SolutionDescriptor {
    let task = EntityDescriptor::new("Task", TypeId::of::<DynamicRow>(), "tasks")
        .with_logical_id(EntityClassId(0))
        .with_variable(
            VariableDescriptor::genuine("worker")
                .with_logical_id(VariableId(0))
                .with_allows_unassigned(false),
        );
    let vehicle = EntityDescriptor::new("Vehicle", TypeId::of::<DynamicRow>(), "vehicles")
        .with_logical_id(EntityClassId(1))
        .with_variable(VariableDescriptor::list("visits").with_logical_id(VariableId(0)));
    SolutionDescriptor::new("DynamicPlan", TypeId::of::<DynamicPlan>())
        .with_entity(task)
        .with_entity(vehicle)
}

fn dynamic_director(solution: DynamicPlan) -> ScoreDirector<DynamicPlan, ()> {
    let descriptor = dynamic_descriptor();
    ScoreDirector::simple(solution, descriptor, |solution, _| {
        solution.scalar_values.len() + solution.list_element_count
    })
}

#[derive(Clone, Debug)]
struct PreferWorkerOne {
    constraint_ref: solverforge_core::ConstraintRef,
}

impl Default for PreferWorkerOne {
    fn default() -> Self {
        Self {
            constraint_ref: solverforge_core::ConstraintRef::new("", "Prefer worker one"),
        }
    }
}

impl solverforge_scoring::IncrementalConstraintSealed for PreferWorkerOne {}

impl solverforge_scoring::IncrementalConstraint<DynamicPlan, SoftScore> for PreferWorkerOne {
    fn evaluate(&self, solution: &DynamicPlan) -> SoftScore {
        SoftScore::of(
            solution
                .scalar_values
                .iter()
                .enumerate()
                .map(|(row, _)| self.row_score(solution, row))
                .sum(),
        )
    }

    fn match_count(&self, solution: &DynamicPlan) -> usize {
        solution
            .scalar_values
            .iter()
            .filter(|value| **value != Some(1))
            .count()
    }

    fn initialize(&mut self, solution: &DynamicPlan) -> SoftScore {
        self.evaluate(solution)
    }

    fn on_insert(
        &mut self,
        solution: &DynamicPlan,
        entity_index: usize,
        descriptor_index: usize,
    ) -> SoftScore {
        if descriptor_index == 0 {
            SoftScore::of(self.row_score(solution, entity_index))
        } else {
            SoftScore::ZERO
        }
    }

    fn on_retract(
        &mut self,
        solution: &DynamicPlan,
        entity_index: usize,
        descriptor_index: usize,
    ) -> SoftScore {
        if descriptor_index == 0 {
            SoftScore::of(-self.row_score(solution, entity_index))
        } else {
            SoftScore::ZERO
        }
    }

    fn reset(&mut self) {}

    fn constraint_ref(&self) -> &solverforge_core::ConstraintRef {
        &self.constraint_ref
    }

    fn weight(&self) -> SoftScore {
        SoftScore::of(-10)
    }
}

impl PreferWorkerOne {
    fn row_score(&self, solution: &DynamicPlan, row: usize) -> i64 {
        if solution.scalar_values.get(row).copied().flatten() == Some(1) {
            0
        } else {
            -10
        }
    }
}

fn dynamic_preference_director(
    solution: DynamicPlan,
) -> ScoreDirector<DynamicPlan, PreferWorkerOne> {
    let descriptor = dynamic_descriptor();
    ScoreDirector::with_descriptor(
        solution,
        PreferWorkerOne::default(),
        descriptor,
        |solution, descriptor_index| match descriptor_index {
            0 => solution.scalar_values.len(),
            1 => solution.lists.len(),
            _ => 0,
        },
    )
}

#[derive(Clone, Debug)]
struct PreferOrderedVisits {
    constraint_ref: solverforge_core::ConstraintRef,
}

impl Default for PreferOrderedVisits {
    fn default() -> Self {
        Self {
            constraint_ref: solverforge_core::ConstraintRef::new("", "Prefer ordered visits"),
        }
    }
}

impl solverforge_scoring::IncrementalConstraintSealed for PreferOrderedVisits {}

impl solverforge_scoring::IncrementalConstraint<DynamicPlan, SoftScore> for PreferOrderedVisits {
    fn evaluate(&self, solution: &DynamicPlan) -> SoftScore {
        SoftScore::of(
            solution
                .lists
                .iter()
                .enumerate()
                .map(|(row, _)| self.row_score(solution, row))
                .sum(),
        )
    }

    fn match_count(&self, solution: &DynamicPlan) -> usize {
        solution
            .lists
            .iter()
            .filter(|list| list.as_slice() != [0, 1])
            .count()
    }

    fn initialize(&mut self, solution: &DynamicPlan) -> SoftScore {
        self.evaluate(solution)
    }

    fn on_insert(
        &mut self,
        solution: &DynamicPlan,
        entity_index: usize,
        descriptor_index: usize,
    ) -> SoftScore {
        if descriptor_index == 1 {
            SoftScore::of(self.row_score(solution, entity_index))
        } else {
            SoftScore::ZERO
        }
    }

    fn on_retract(
        &mut self,
        solution: &DynamicPlan,
        entity_index: usize,
        descriptor_index: usize,
    ) -> SoftScore {
        if descriptor_index == 1 {
            SoftScore::of(-self.row_score(solution, entity_index))
        } else {
            SoftScore::ZERO
        }
    }

    fn reset(&mut self) {}

    fn constraint_ref(&self) -> &solverforge_core::ConstraintRef {
        &self.constraint_ref
    }

    fn weight(&self) -> SoftScore {
        SoftScore::of(-10)
    }
}

impl PreferOrderedVisits {
    fn row_score(&self, solution: &DynamicPlan, row: usize) -> i64 {
        if solution
            .lists
            .get(row)
            .is_some_and(|list| list.as_slice() == [0, 1])
        {
            0
        } else {
            -10
        }
    }
}

fn dynamic_ordered_visits_director(
    solution: DynamicPlan,
) -> ScoreDirector<DynamicPlan, PreferOrderedVisits> {
    let descriptor = dynamic_descriptor();
    ScoreDirector::with_descriptor(
        solution,
        PreferOrderedVisits::default(),
        descriptor,
        |solution, descriptor_index| match descriptor_index {
            0 => solution.scalar_values.len(),
            1 => solution.lists.len(),
            _ => 0,
        },
    )
}

#[test]
fn dynamic_scalar_slot_runs_through_default_construction() {
    let descriptor = dynamic_descriptor();
    let scalar = DynamicScalarVariableSlot::new(
        EntityClassId(0),
        VariableId(0),
        "Task",
        "worker",
        false,
    );
    let model: RuntimeModel<
        DynamicPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicScalar(scalar)]);
    let mut phase = Construction::new(None, descriptor, model);
    let plan = DynamicPlan {
        score: None,
        scalar_values: vec![None, None],
        scalar_candidates: vec![vec![1], vec![2]],
        lists: Vec::new(),
        list_element_count: 0,
    };
    let mut solver_scope = SolverScope::new(dynamic_director(plan));

    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.working_solution().scalar_values,
        vec![Some(1), Some(2)]
    );
}

#[test]
fn dynamic_scalar_slot_runs_through_local_search() {
    let descriptor = dynamic_descriptor();
    let scalar = DynamicScalarVariableSlot::new(
        EntityClassId(0),
        VariableId(0),
        "Task",
        "worker",
        false,
    );
    let model: RuntimeModel<
        DynamicPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicScalar(scalar)]);
    let config = solverforge_config::SolverConfig {
        phases: vec![
            solverforge_config::PhaseConfig::ConstructionHeuristic(
                solverforge_config::ConstructionHeuristicConfig::default(),
            ),
            solverforge_config::PhaseConfig::LocalSearch(solverforge_config::LocalSearchConfig {
                local_search_type: solverforge_config::LocalSearchType::VariableNeighborhoodDescent,
                neighborhoods: vec![solverforge_config::MoveSelectorConfig::ChangeMoveSelector(
                    solverforge_config::ChangeMoveConfig {
                        value_candidate_limit: None,
                        target: solverforge_config::VariableTargetConfig {
                            entity_class: Some("Task".to_string()),
                            variable_name: Some("worker".to_string()),
                        },
                    },
                )],
                termination: Some(solverforge_config::TerminationConfig {
                    step_count_limit: Some(4),
                    ..solverforge_config::TerminationConfig::default()
                }),
                ..solverforge_config::LocalSearchConfig::default()
            }),
        ],
        ..solverforge_config::SolverConfig::default()
    };
    let mut phases = super::build_phases(&config, &descriptor, &model);
    let plan = DynamicPlan {
        score: None,
        scalar_values: vec![None],
        scalar_candidates: vec![vec![0, 1]],
        lists: Vec::new(),
        list_element_count: 0,
    };
    let mut solver_scope = SolverScope::new(dynamic_preference_director(plan));

    phases.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().scalar_values, vec![Some(1)]);
    assert_eq!(solver_scope.current_score().copied(), Some(SoftScore::of(0)));
}

#[test]
fn dynamic_list_slot_runs_through_local_search() {
    let descriptor = dynamic_descriptor();
    let list = DynamicListVariableSlot::new(EntityClassId(1), VariableId(0), "Vehicle", "visits");
    let model: RuntimeModel<
        DynamicPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicList(list)]);
    let config = solverforge_config::SolverConfig {
        phases: vec![solverforge_config::PhaseConfig::LocalSearch(
            solverforge_config::LocalSearchConfig {
                local_search_type: solverforge_config::LocalSearchType::VariableNeighborhoodDescent,
                neighborhoods: vec![solverforge_config::MoveSelectorConfig::ListChangeMoveSelector(
                    solverforge_config::ListChangeMoveConfig {
                        target: solverforge_config::VariableTargetConfig {
                            entity_class: Some("Vehicle".to_string()),
                            variable_name: Some("visits".to_string()),
                        },
                    },
                )],
                termination: Some(solverforge_config::TerminationConfig {
                    step_count_limit: Some(4),
                    ..solverforge_config::TerminationConfig::default()
                }),
                ..solverforge_config::LocalSearchConfig::default()
            },
        )],
        ..solverforge_config::SolverConfig::default()
    };
    let mut phases = super::build_phases(&config, &descriptor, &model);
    let plan = DynamicPlan {
        score: None,
        scalar_values: Vec::new(),
        scalar_candidates: Vec::new(),
        lists: vec![vec![1, 0]],
        list_element_count: 2,
    };
    let mut solver_scope = SolverScope::new(dynamic_ordered_visits_director(plan));

    phases.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().lists, vec![vec![0, 1]]);
    assert_eq!(solver_scope.current_score().copied(), Some(SoftScore::of(0)));
}

#[test]
fn dynamic_list_slot_runs_through_default_construction() {
    let descriptor = dynamic_descriptor();
    let list = DynamicListVariableSlot::new(EntityClassId(1), VariableId(0), "Vehicle", "visits");
    let model: RuntimeModel<
        DynamicPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicList(list)]);
    let mut phase = Construction::new(None, descriptor, model);
    let plan = DynamicPlan {
        score: None,
        scalar_values: Vec::new(),
        scalar_candidates: Vec::new(),
        lists: vec![Vec::new(), Vec::new()],
        list_element_count: 3,
    };
    let mut solver_scope = SolverScope::new(dynamic_director(plan));

    phase.solve(&mut solver_scope);

    let assigned = solver_scope
        .working_solution()
        .lists
        .iter()
        .flat_map(|list| list.iter().copied())
        .collect::<Vec<_>>();
    assert_eq!(assigned.len(), 3);
    assert!(assigned.contains(&0));
    assert!(assigned.contains(&1));
    assert!(assigned.contains(&2));
}
