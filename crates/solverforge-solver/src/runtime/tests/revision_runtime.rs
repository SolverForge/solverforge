
#[derive(Clone, Debug)]
struct RevisionWorker;

#[derive(Clone, Debug)]
struct RevisionTask {
    worker_idx: Option<usize>,
}

#[derive(Clone, Debug)]
struct RevisionPlan {
    score: Option<SoftScore>,
    workers: Vec<RevisionWorker>,
    tasks: Vec<RevisionTask>,
    routes: Vec<Vec<usize>>,
    route_pool: Vec<usize>,
}

#[derive(Clone, Debug)]
struct RevisionDirector {
    working_solution: RevisionPlan,
    descriptor: SolutionDescriptor,
}

impl PlanningSolution for RevisionPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Director<RevisionPlan> for RevisionDirector {
    fn working_solution(&self) -> &RevisionPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut RevisionPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let route_ready = !self.working_solution.routes[0].is_empty();
        let assigned = self.working_solution.tasks[0].worker_idx.is_some();
        let score = match (route_ready, assigned) {
            (false, false) => SoftScore::of(0),
            (false, true) => SoftScore::of(-1),
            (true, false) => SoftScore::of(0),
            (true, true) => SoftScore::of(10),
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> RevisionPlan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        match descriptor_index {
            0 => Some(self.working_solution.tasks.len()),
            1 => Some(self.working_solution.routes.len()),
            _ => None,
        }
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.tasks.len() + self.working_solution.routes.len())
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

fn revision_task_getter(entity: &dyn std::any::Any) -> Option<usize> {
    entity
        .downcast_ref::<RevisionTask>()
        .expect("task expected")
        .worker_idx
}

fn revision_task_setter(entity: &mut dyn std::any::Any, value: Option<usize>) {
    entity
        .downcast_mut::<RevisionTask>()
        .expect("task expected")
        .worker_idx = value;
}

fn revision_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("RevisionPlan", TypeId::of::<RevisionPlan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<RevisionTask>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |solution: &RevisionPlan| &solution.tasks,
                    |solution: &mut RevisionPlan| &mut solution.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(revision_task_getter, revision_task_setter),
                ),
        )
        .with_entity(
            EntityDescriptor::new("Route", TypeId::of::<Vec<usize>>(), "routes").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Route",
                    "routes",
                    |solution: &RevisionPlan| &solution.routes,
                    |solution: &mut RevisionPlan| &mut solution.routes,
                )),
            ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<RevisionWorker>(), "workers")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |solution: &RevisionPlan| &solution.workers,
                    |solution: &mut RevisionPlan| &mut solution.workers,
                ))),
        )
}

fn revision_task_count(solution: &RevisionPlan) -> usize {
    solution.tasks.len()
}

fn revision_worker_count(solution: &RevisionPlan, _provider_index: usize) -> usize {
    solution.workers.len()
}

fn revision_worker_get(
    solution: &RevisionPlan,
    entity_index: usize,
    _variable_index: usize,
) -> Option<usize> {
    solution.tasks[entity_index].worker_idx
}

fn revision_worker_set(
    solution: &mut RevisionPlan,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.tasks[entity_index].worker_idx = value;
}

fn revision_route_count(solution: &RevisionPlan) -> usize {
    solution.routes.len()
}

fn revision_route_element_count(solution: &RevisionPlan) -> usize {
    solution.route_pool.len()
}

fn revision_assigned_route_elements(solution: &RevisionPlan) -> Vec<usize> {
    solution
        .routes
        .iter()
        .flat_map(|route| route.iter().copied())
        .collect()
}

fn revision_route_len(solution: &RevisionPlan, entity_index: usize) -> usize {
    solution.routes[entity_index].len()
}

fn revision_route_remove(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
) -> Option<usize> {
    let route = solution.routes.get_mut(entity_index)?;
    (pos < route.len()).then(|| route.remove(pos))
}

fn revision_route_remove_for_construction(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].remove(pos)
}

fn revision_route_insert(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].insert(pos, value);
}

fn revision_route_get(solution: &RevisionPlan, entity_index: usize, pos: usize) -> Option<usize> {
    solution.routes[entity_index].get(pos).copied()
}

fn revision_route_set(solution: &mut RevisionPlan, entity_index: usize, pos: usize, value: usize) {
    solution.routes[entity_index][pos] = value;
}

fn revision_route_reverse(
    solution: &mut RevisionPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) {
    solution.routes[entity_index][start..end].reverse();
}

fn revision_route_sublist_remove(
    solution: &mut RevisionPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    solution.routes[entity_index].drain(start..end).collect()
}

fn revision_route_sublist_insert(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
    values: Vec<usize>,
) {
    solution.routes[entity_index].splice(pos..pos, values);
}

fn revision_route_ruin_remove(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].remove(pos)
}

fn revision_route_ruin_insert(
    solution: &mut RevisionPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].insert(pos, value);
}

fn revision_route_index_to_element(solution: &RevisionPlan, idx: usize) -> usize {
    solution.route_pool[idx]
}

fn revision_model() -> RuntimeModel<RevisionPlan, usize, DefaultMeter, DefaultMeter> {
    RuntimeModel::new(vec![
        VariableSlot::Scalar(ScalarVariableSlot::new(
            0,
            0,
            "Task",
            revision_task_count,
            "worker_idx",
            revision_worker_get,
            revision_worker_set,
            ValueSource::SolutionCount {
                count_fn: revision_worker_count,
                provider_index: 0,
            },
            true,
        )),
        VariableSlot::List(ListVariableSlot::new(
            "Route",
            revision_route_element_count,
            revision_assigned_route_elements,
            revision_route_len,
            revision_route_remove,
            revision_route_remove_for_construction,
            revision_route_insert,
            revision_route_get,
            revision_route_set,
            revision_route_reverse,
            revision_route_sublist_remove,
            revision_route_sublist_insert,
            revision_route_ruin_remove,
            revision_route_ruin_insert,
            revision_route_index_to_element,
            revision_route_count,
            DefaultMeter::default(),
            DefaultMeter::default(),
            "visits",
            1,
            None,
            None,
            None,
            None,
            None,
            None,
        )),
    ])
}

#[test]
fn generic_mixed_phase_reopens_optional_none_after_list_commit() {
    let descriptor = revision_descriptor();
    let plan = RevisionPlan {
        score: None,
        workers: vec![RevisionWorker],
        tasks: vec![RevisionTask { worker_idx: None }],
        routes: vec![Vec::new()],
        route_pool: vec![10],
    };
    let director = RevisionDirector {
        working_solution: plan,
        descriptor: descriptor.clone(),
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(None, descriptor, revision_model());
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().routes[0], vec![10]);
    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert!(solver_scope.stats().moves_accepted >= 1);
}
