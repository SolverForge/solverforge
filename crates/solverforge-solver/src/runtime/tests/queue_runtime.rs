
#[derive(Clone, Debug)]
struct QueueRuntimeWorker;

#[derive(Clone, Debug)]
struct QueueRuntimeTask {
    worker_idx: Option<usize>,
    preferred_worker: usize,
    allowed_workers: Vec<usize>,
}

#[derive(Clone, Debug)]
struct QueueRuntimePlan {
    score: Option<SoftScore>,
    workers: Vec<QueueRuntimeWorker>,
    tasks: Vec<QueueRuntimeTask>,
    assignment_log: Vec<usize>,
}

impl PlanningSolution for QueueRuntimePlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct QueueRuntimeDirector {
    working_solution: QueueRuntimePlan,
    descriptor: SolutionDescriptor,
}

impl QueueRuntimeDirector {
    fn new(working_solution: QueueRuntimePlan, descriptor: SolutionDescriptor) -> Self {
        Self {
            working_solution,
            descriptor,
        }
    }
}

impl Director<QueueRuntimePlan> for QueueRuntimeDirector {
    fn working_solution(&self) -> &QueueRuntimePlan {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut QueueRuntimePlan {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = SoftScore::of(0);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> QueueRuntimePlan {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, entity_index: usize) {
        self.working_solution.assignment_log.push(entity_index);
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.tasks.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.tasks.len())
    }

    fn constraint_metadata(&self) -> &[solverforge_scoring::ConstraintMetadata] {
        &[]
    }
}

fn queue_runtime_get_worker_idx(entity: &dyn std::any::Any) -> Option<usize> {
    entity
        .downcast_ref::<QueueRuntimeTask>()
        .expect("queue task expected")
        .worker_idx
}

fn queue_runtime_set_worker_idx(entity: &mut dyn std::any::Any, value: Option<usize>) {
    entity
        .downcast_mut::<QueueRuntimeTask>()
        .expect("queue task expected")
        .worker_idx = value;
}

fn queue_runtime_allowed_workers(entity: &dyn std::any::Any) -> Vec<usize> {
    entity
        .downcast_ref::<QueueRuntimeTask>()
        .expect("queue task expected")
        .allowed_workers
        .clone()
}

fn queue_runtime_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("QueueRuntimePlan", TypeId::of::<QueueRuntimePlan>())
        .with_entity(
            EntityDescriptor::new("QueueTask", TypeId::of::<QueueRuntimeTask>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "QueueTask",
                    "tasks",
                    |solution: &QueueRuntimePlan| &solution.tasks,
                    |solution: &mut QueueRuntimePlan| &mut solution.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_usize_accessors(
                            queue_runtime_get_worker_idx,
                            queue_runtime_set_worker_idx,
                        )
                        .with_entity_value_provider(queue_runtime_allowed_workers),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new(
                "QueueRuntimeWorker",
                TypeId::of::<QueueRuntimeWorker>(),
                "workers",
            )
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "QueueRuntimeWorker",
                "workers",
                |solution: &QueueRuntimePlan| &solution.workers,
                |solution: &mut QueueRuntimePlan| &mut solution.workers,
            ))),
        )
}

fn queue_runtime_task_count(solution: &QueueRuntimePlan) -> usize {
    solution.tasks.len()
}

fn queue_runtime_worker_get(
    solution: &QueueRuntimePlan,
    entity_index: usize,
    _variable_index: usize,
) -> Option<usize> {
    solution.tasks[entity_index].worker_idx
}

fn queue_runtime_worker_set(
    solution: &mut QueueRuntimePlan,
    entity_index: usize,
    _variable_index: usize,
    value: Option<usize>,
) {
    solution.tasks[entity_index].worker_idx = value;
}

fn queue_runtime_allowed_workers_for_entity(
    solution: &QueueRuntimePlan,
    entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    &solution.tasks[entity_index].allowed_workers
}

fn queue_runtime_entity_order_key(
    solution: &QueueRuntimePlan,
    entity_index: usize,
    _variable_index: usize,
) -> Option<i64> {
    let preferred_worker = solution.tasks[entity_index].preferred_worker;
    Some(
        solution
            .tasks
            .iter()
            .filter(|task| task.worker_idx == Some(preferred_worker))
            .count() as i64,
    )
}

fn queue_runtime_value_load_key(
    solution: &QueueRuntimePlan,
    _entity_index: usize,
    _variable_index: usize,
    value: usize,
) -> Option<i64> {
    Some(
        solution
            .tasks
            .iter()
            .filter(|task| task.worker_idx == Some(value))
            .count() as i64,
    )
}

fn queue_runtime_model(
    entity_order_key: Option<fn(&QueueRuntimePlan, usize, usize) -> Option<i64>>,
    value_order_key: Option<fn(&QueueRuntimePlan, usize, usize, usize) -> Option<i64>>,
) -> ModelContext<QueueRuntimePlan, usize, DefaultMeter, DefaultMeter> {
    let mut ctx = ScalarVariableContext::new(
        0,
        0,
        "QueueTask",
        queue_runtime_task_count,
        "worker_idx",
        queue_runtime_worker_get,
        queue_runtime_worker_set,
        ValueSource::EntitySlice {
            values_for_entity: queue_runtime_allowed_workers_for_entity,
        },
        false,
    );
    if let Some(order_key) = entity_order_key {
        ctx = ctx.with_construction_entity_order_key(order_key);
    }
    if let Some(order_key) = value_order_key {
        ctx = ctx.with_construction_value_order_key(order_key);
    }
    ModelContext::new(vec![VariableContext::Scalar(ctx)])
}

#[test]
fn queue_runtime_allocate_entity_from_queue_uses_model_hook_without_descriptor_keys() {
    let descriptor = queue_runtime_descriptor();
    let plan = QueueRuntimePlan {
        score: None,
        workers: vec![QueueRuntimeWorker, QueueRuntimeWorker],
        tasks: vec![
            QueueRuntimeTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0],
            },
            QueueRuntimeTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0],
            },
            QueueRuntimeTask {
                worker_idx: None,
                preferred_worker: 1,
                allowed_workers: vec![1],
            },
        ],
        assignment_log: Vec::new(),
    };
    let director = QueueRuntimeDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(config(
            ConstructionHeuristicType::AllocateEntityFromQueue,
            Some("QueueTask"),
            Some("worker_idx"),
        )),
        descriptor,
        queue_runtime_model(Some(queue_runtime_entity_order_key), None),
    );
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.working_solution().assignment_log,
        vec![0, 2, 1]
    );
}

#[test]
fn queue_runtime_allocate_to_value_from_queue_uses_model_hook_without_descriptor_keys() {
    let descriptor = queue_runtime_descriptor();
    let plan = QueueRuntimePlan {
        score: None,
        workers: vec![QueueRuntimeWorker, QueueRuntimeWorker],
        tasks: vec![
            QueueRuntimeTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0, 1],
            },
            QueueRuntimeTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0, 1],
            },
        ],
        assignment_log: Vec::new(),
    };
    let director = QueueRuntimeDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = Construction::new(
        Some(config(
            ConstructionHeuristicType::AllocateToValueFromQueue,
            Some("QueueTask"),
            Some("worker_idx"),
        )),
        descriptor,
        queue_runtime_model(None, Some(queue_runtime_value_load_key)),
    );
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope
            .working_solution()
            .tasks
            .iter()
            .map(|task| task.worker_idx)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)]
    );
}
