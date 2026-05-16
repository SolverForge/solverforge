
#[derive(Clone, Debug)]
struct MixedTargetWorker;

#[derive(Clone, Debug)]
struct MixedTargetRoute {
    worker_idx: Option<usize>,
    tasks: Vec<usize>,
}

#[derive(Clone, Debug)]
struct MixedTargetPlan {
    score: Option<SoftScore>,
    workers: Vec<MixedTargetWorker>,
    routes: Vec<MixedTargetRoute>,
    task_pool: Vec<usize>,
}

impl PlanningSolution for MixedTargetPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct MixedTargetDirector {
    working_solution: MixedTargetPlan,
    descriptor: SolutionDescriptor,
    score_mode: MixedTargetScoreMode,
}

#[derive(Clone, Copy, Debug)]
enum MixedTargetScoreMode {
    Flat,
    PreferAssignedWorker,
}

impl Director<MixedTargetPlan> for MixedTargetDirector {
    fn working_solution(&self) -> &MixedTargetPlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut MixedTargetPlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = match self.score_mode {
            MixedTargetScoreMode::Flat => SoftScore::of(0),
            MixedTargetScoreMode::PreferAssignedWorker => SoftScore::of(
                self.working_solution.routes[0]
                    .worker_idx
                    .map(|_| 1)
                    .unwrap_or(0),
            ),
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> MixedTargetPlan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.routes.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.routes.len())
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

fn mixed_target_plan() -> MixedTargetPlan {
    MixedTargetPlan {
        score: None,
        workers: vec![MixedTargetWorker],
        routes: vec![MixedTargetRoute {
            worker_idx: None,
            tasks: Vec::new(),
        }],
        task_pool: vec![10],
    }
}

fn mixed_target_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("MixedTargetPlan", TypeId::of::<MixedTargetPlan>())
        .with_entity(
            EntityDescriptor::new("Route", TypeId::of::<MixedTargetRoute>(), "routes")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Route",
                    "routes",
                    |solution: &MixedTargetPlan| &solution.routes,
                    |solution: &mut MixedTargetPlan| &mut solution.routes,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(
                            mixed_target_worker_get_any,
                            mixed_target_worker_set_any,
                        ),
                )
                .with_variable(VariableDescriptor::list("tasks")),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<MixedTargetWorker>(), "workers")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |solution: &MixedTargetPlan| &solution.workers,
                    |solution: &mut MixedTargetPlan| &mut solution.workers,
                ))),
        )
}

fn mixed_target_route_count(solution: &MixedTargetPlan) -> usize {
    solution.routes.len()
}

fn mixed_target_worker_get(
    solution: &MixedTargetPlan,
    entity_index: usize,
    _variable_index: usize,
) -> Option<usize> {
    solution.routes[entity_index].worker_idx
}

fn mixed_target_worker_set(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.routes[entity_index].worker_idx = value;
}

fn mixed_target_worker_get_any(entity: &dyn std::any::Any) -> Option<usize> {
    entity
        .downcast_ref::<MixedTargetRoute>()
        .expect("route expected")
        .worker_idx
}

fn mixed_target_worker_set_any(entity: &mut dyn std::any::Any, value: Option<usize>) {
    entity
        .downcast_mut::<MixedTargetRoute>()
        .expect("route expected")
        .worker_idx = value;
}

fn mixed_target_element_count(solution: &MixedTargetPlan) -> usize {
    solution.task_pool.len()
}

fn mixed_target_assigned_elements(solution: &MixedTargetPlan) -> Vec<usize> {
    solution
        .routes
        .iter()
        .flat_map(|route| route.tasks.iter().copied())
        .collect()
}

fn mixed_target_list_len(solution: &MixedTargetPlan, entity_index: usize) -> usize {
    solution.routes[entity_index].tasks.len()
}

fn mixed_target_list_remove(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
) -> Option<usize> {
    let route = solution.routes.get_mut(entity_index)?;
    (pos < route.tasks.len()).then(|| route.tasks.remove(pos))
}

fn mixed_target_construction_list_remove(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].tasks.remove(pos)
}

fn mixed_target_list_insert(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].tasks.insert(pos, value);
}

fn mixed_target_list_get(
    solution: &MixedTargetPlan,
    entity_index: usize,
    pos: usize,
) -> Option<usize> {
    solution.routes[entity_index].tasks.get(pos).copied()
}

fn mixed_target_list_set(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].tasks[pos] = value;
}

fn mixed_target_list_reverse(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) {
    solution.routes[entity_index].tasks[start..end].reverse();
}

fn mixed_target_sublist_remove(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    start: usize,
    end: usize,
) -> Vec<usize> {
    solution.routes[entity_index]
        .tasks
        .drain(start..end)
        .collect()
}

fn mixed_target_sublist_insert(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
    values: Vec<usize>,
) {
    solution.routes[entity_index].tasks.splice(pos..pos, values);
}

fn mixed_target_ruin_remove(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
) -> usize {
    solution.routes[entity_index].tasks.remove(pos)
}

fn mixed_target_ruin_insert(
    solution: &mut MixedTargetPlan,
    entity_index: usize,
    pos: usize,
    value: usize,
) {
    solution.routes[entity_index].tasks.insert(pos, value);
}

fn mixed_target_index_to_element(solution: &MixedTargetPlan, idx: usize) -> usize {
    solution.task_pool[idx]
}

static MIXED_TARGET_AVAILABLE_WORKERS: [usize; 1] = [0];

fn mixed_target_available_workers(
    solution: &MixedTargetPlan,
    entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    if solution.routes[entity_index].tasks.is_empty() {
        &MIXED_TARGET_AVAILABLE_WORKERS
    } else {
        &[]
    }
}

fn mixed_target_model() -> RuntimeModel<MixedTargetPlan, usize, DefaultMeter, DefaultMeter> {
    RuntimeModel::new(vec![
        VariableSlot::Scalar(ScalarVariableSlot::new(
            0,
            0,
            "Route",
            mixed_target_route_count,
            "worker_idx",
            mixed_target_worker_get,
            mixed_target_worker_set,
            ValueSource::EntitySlice {
                values_for_entity: mixed_target_available_workers,
            },
            true,
        )),
        VariableSlot::List(ListVariableSlot::new(
            "Route",
            mixed_target_element_count,
            mixed_target_assigned_elements,
            mixed_target_list_len,
            mixed_target_list_remove,
            mixed_target_construction_list_remove,
            mixed_target_list_insert,
            mixed_target_list_get,
            mixed_target_list_set,
            mixed_target_list_reverse,
            mixed_target_sublist_remove,
            mixed_target_sublist_insert,
            mixed_target_ruin_remove,
            mixed_target_ruin_insert,
            mixed_target_index_to_element,
            mixed_target_route_count,
            DefaultMeter::default(),
            DefaultMeter::default(),
            "tasks",
            0,
            None,
            None,
            None,
            None,
            None,
            None,
        )),
    ])
}

fn solve_mixed_target_construction(
    kind: ConstructionHeuristicType,
    entity_class: Option<&str>,
    score_mode: MixedTargetScoreMode,
) -> MixedTargetPlan {
    let descriptor = mixed_target_descriptor();
    let director = MixedTargetDirector {
        working_solution: mixed_target_plan(),
        descriptor: descriptor.clone(),
        score_mode,
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(config(kind, entity_class, None)),
        descriptor,
        mixed_target_model(),
    );
    phase.solve(&mut solver_scope);
    solver_scope.working_solution().clone()
}

#[test]
fn entity_class_target_matches_scalar_and_list_in_same_owner() {
    let solution = solve_mixed_target_construction(
        ConstructionHeuristicType::FirstFit,
        Some("Route"),
        MixedTargetScoreMode::PreferAssignedWorker,
    );

    assert_eq!(solution.routes[0].worker_idx, Some(0));
    assert_eq!(solution.routes[0].tasks, vec![10]);
}

#[test]
fn mixed_cheapest_insertion_breaks_equal_scores_by_canonical_order() {
    let solution = solve_mixed_target_construction(
        ConstructionHeuristicType::CheapestInsertion,
        Some("Route"),
        MixedTargetScoreMode::Flat,
    );

    assert_eq!(solution.routes[0].worker_idx, Some(0));
    assert_eq!(solution.routes[0].tasks, vec![10]);
}
