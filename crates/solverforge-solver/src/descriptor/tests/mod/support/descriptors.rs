fn queue_descriptor(
    entity_order_key: Option<solverforge_core::domain::UsizeConstructionEntityOrderKey>,
    value_order_key: Option<solverforge_core::domain::UsizeConstructionValueOrderKey>,
) -> SolutionDescriptor {
    let mut variable = VariableDescriptor::genuine("worker_idx")
        .with_allows_unassigned(false)
        .with_usize_accessors(queue_get_worker_idx, queue_set_worker_idx)
        .with_entity_value_provider(queue_allowed_workers);
    if let Some(order_key) = entity_order_key {
        variable = variable.with_construction_entity_order_key(order_key);
    }
    if let Some(order_key) = value_order_key {
        variable = variable.with_construction_value_order_key(order_key);
    }

    SolutionDescriptor::new("QueuePlan", TypeId::of::<QueuePlan>())
        .with_entity(
            EntityDescriptor::new("QueueTask", TypeId::of::<QueueTask>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "QueueTask",
                    "tasks",
                    |s: &QueuePlan| &s.tasks,
                    |s: &mut QueuePlan| &mut s.tasks,
                )))
                .with_variable(variable),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<Worker>(), "workers").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |s: &QueuePlan| &s.workers,
                    |s: &mut QueuePlan| &mut s.workers,
                )),
            ),
        )
}

fn descriptor_with_allows_unassigned(allows_unassigned: bool) -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |s: &Plan| &s.tasks,
                    |s: &mut Plan| &mut s.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(allows_unassigned)
                        .with_value_range("workers")
                        .with_usize_accessors(get_worker_idx, set_worker_idx),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<Worker>(), "workers").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |s: &Plan| &s.workers,
                    |s: &mut Plan| &mut s.workers,
                )),
            ),
        )
}

fn descriptor_without_value_range() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "Task",
                "tasks",
                |s: &Plan| &s.tasks,
                |s: &mut Plan| &mut s.tasks,
            )))
            .with_variable(
                VariableDescriptor::genuine("worker_idx")
                    .with_allows_unassigned(false)
                    .with_usize_accessors(get_worker_idx, set_worker_idx),
            ),
    )
}

fn descriptor_with_empty_countable_range() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "Task",
                "tasks",
                |s: &Plan| &s.tasks,
                |s: &mut Plan| &mut s.tasks,
            )))
            .with_variable(
                VariableDescriptor::genuine("worker_idx")
                    .with_allows_unassigned(false)
                    .with_value_range_type(ValueRangeType::CountableRange { from: 0, to: 0 })
                    .with_usize_accessors(get_worker_idx, set_worker_idx),
            ),
    )
}

fn descriptor() -> SolutionDescriptor {
    descriptor_with_allows_unassigned(true)
}

fn descriptor_with_nearby_value_meter() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |s: &Plan| &s.tasks,
                    |s: &mut Plan| &mut s.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(get_worker_idx, set_worker_idx)
                        .with_nearby_value_candidates(nearby_worker_candidates)
                        .with_nearby_value_distance_meter(nearby_worker_value_distance),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<Worker>(), "workers").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |s: &Plan| &s.workers,
                    |s: &mut Plan| &mut s.workers,
                )),
            ),
        )
}

fn descriptor_with_nearby_value_meter_only() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |s: &Plan| &s.tasks,
                    |s: &mut Plan| &mut s.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(get_worker_idx, set_worker_idx)
                        .with_nearby_value_distance_meter(nearby_worker_value_distance),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<Worker>(), "workers").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |s: &Plan| &s.workers,
                    |s: &mut Plan| &mut s.workers,
                )),
            ),
        )
}

fn descriptor_with_nearby_entity_meter() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |s: &Plan| &s.tasks,
                    |s: &mut Plan| &mut s.tasks,
                )))
                .with_variable(
                    VariableDescriptor::genuine("worker_idx")
                        .with_allows_unassigned(true)
                        .with_value_range("workers")
                        .with_usize_accessors(get_worker_idx, set_worker_idx)
                        .with_nearby_entity_candidates(nearby_task_candidates)
                        .with_nearby_entity_distance_meter(nearby_worker_entity_distance),
                ),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<Worker>(), "workers").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |s: &Plan| &s.workers,
                    |s: &mut Plan| &mut s.workers,
                )),
            ),
        )
}

fn restricted_descriptor_with_variable(variable: VariableDescriptor) -> SolutionDescriptor {
    SolutionDescriptor::new("RestrictedPlan", TypeId::of::<RestrictedPlan>())
        .with_entity(
            EntityDescriptor::new("Task", TypeId::of::<RestrictedTask>(), "tasks")
                .with_extractor(Box::new(EntityCollectionExtractor::new(
                    "Task",
                    "tasks",
                    |s: &RestrictedPlan| &s.tasks,
                    |s: &mut RestrictedPlan| &mut s.tasks,
                )))
                .with_variable(variable),
        )
        .with_problem_fact(
            ProblemFactDescriptor::new("Worker", TypeId::of::<Worker>(), "workers").with_extractor(
                Box::new(EntityCollectionExtractor::new(
                    "Worker",
                    "workers",
                    |s: &RestrictedPlan| &s.workers,
                    |s: &mut RestrictedPlan| &mut s.workers,
                )),
            ),
        )
}

fn restricted_variable() -> VariableDescriptor {
    VariableDescriptor::genuine("worker_idx")
        .with_allows_unassigned(true)
        .with_usize_accessors(restricted_get_worker_idx, restricted_set_worker_idx)
        .with_entity_value_provider(restricted_allowed_workers)
}

fn restricted_panic_after_index_variable() -> VariableDescriptor {
    VariableDescriptor::genuine("worker_idx")
        .with_allows_unassigned(true)
        .with_usize_accessors(restricted_get_worker_idx, restricted_set_worker_idx)
        .with_entity_value_provider(restricted_allowed_workers_panic_after_index)
}

fn restricted_descriptor() -> SolutionDescriptor {
    restricted_descriptor_with_variable(restricted_variable())
}

fn restricted_descriptor_with_nearby_entity_meter() -> SolutionDescriptor {
    restricted_descriptor_with_variable(
        restricted_variable()
            .with_nearby_entity_candidates(restricted_nearby_task_candidates)
            .with_nearby_entity_distance_meter(nearby_worker_entity_distance),
    )
}
