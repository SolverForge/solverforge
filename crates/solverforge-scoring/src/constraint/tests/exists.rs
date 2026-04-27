use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::ConstraintFactory;

#[derive(Clone)]
struct Task {
    assignee: Option<usize>,
}

#[derive(Clone)]
struct Worker {
    id: usize,
    available: bool,
}

#[derive(Clone)]
struct TaskSchedule {
    tasks: Vec<Task>,
    workers: Vec<Worker>,
}

fn tasks(schedule: &TaskSchedule) -> &[Task] {
    schedule.tasks.as_slice()
}

fn workers(schedule: &TaskSchedule) -> &[Worker] {
    schedule.workers.as_slice()
}

#[test]
fn test_exists_updates_all_matching_a_entities_when_b_descriptor_changes() {
    let mut constraint = ConstraintFactory::<TaskSchedule, SoftScore>::new()
        .for_each(source(
            tasks as fn(&TaskSchedule) -> &[Task],
            ChangeSource::Static,
        ))
        .filter(|task: &Task| task.assignee.is_some())
        .if_exists((
            ConstraintFactory::<TaskSchedule, SoftScore>::new()
                .for_each(source(
                    workers as fn(&TaskSchedule) -> &[Worker],
                    ChangeSource::Descriptor(0),
                ))
                .filter(|worker: &Worker| !worker.available),
            equal_bi(
                |task: &Task| task.assignee,
                |worker: &Worker| Some(worker.id),
            ),
        ))
        .penalize(SoftScore::of(1))
        .named("unavailable worker");

    let mut schedule = TaskSchedule {
        tasks: vec![
            Task { assignee: Some(0) },
            Task { assignee: Some(0) },
            Task { assignee: Some(1) },
        ],
        workers: vec![
            Worker {
                id: 0,
                available: true,
            },
            Worker {
                id: 1,
                available: true,
            },
        ],
    };

    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(0));

    total = total + constraint.on_retract(&schedule, 0, 0);
    schedule.workers[0].available = false;
    total = total + constraint.on_insert(&schedule, 0, 0);

    assert_eq!(total, constraint.evaluate(&schedule));
    assert_eq!(total, SoftScore::of(-2));
}

#[derive(Clone)]
struct CustomerState {
    customers: Vec<usize>,
    routes: Vec<Vec<usize>>,
}

fn customers(state: &CustomerState) -> &[usize] {
    state.customers.as_slice()
}

fn routes(state: &CustomerState) -> &[Vec<usize>] {
    state.routes.as_slice()
}

#[test]
fn test_flattened_not_exists_updates_all_matching_a_entities_when_route_changes() {
    let mut constraint = ConstraintFactory::<CustomerState, SoftScore>::new()
        .for_each(source(
            customers as fn(&CustomerState) -> &[usize],
            ChangeSource::Static,
        ))
        .if_not_exists((
            ConstraintFactory::<CustomerState, SoftScore>::new()
                .for_each(source(
                    routes as fn(&CustomerState) -> &[Vec<usize>],
                    ChangeSource::Descriptor(0),
                ))
                .flattened(|route: &Vec<usize>| route),
            equal_bi(|customer: &usize| *customer, |assigned: &usize| *assigned),
        ))
        .penalize(SoftScore::of(1))
        .named("missing assignment");

    let mut state = CustomerState {
        customers: vec![1, 2, 3],
        routes: vec![Vec::new()],
    };

    let mut total = constraint.initialize(&state);
    assert_eq!(total, SoftScore::of(-3));

    total = total + constraint.on_retract(&state, 0, 0);
    state.routes[0] = vec![1, 2, 3];
    total = total + constraint.on_insert(&state, 0, 0);

    assert_eq!(total, constraint.evaluate(&state));
    assert_eq!(total, SoftScore::of(0));
}

#[derive(Clone)]
struct TaggedItem {
    key: usize,
    enabled: bool,
}

#[derive(Clone)]
struct TaggedItems {
    items: Vec<TaggedItem>,
}

fn tagged_items(state: &TaggedItems) -> &[TaggedItem] {
    state.items.as_slice()
}

#[test]
fn test_exists_same_source_updates_consistently() {
    let mut constraint = ConstraintFactory::<TaggedItems, SoftScore>::new()
        .for_each(source(
            tagged_items as fn(&TaggedItems) -> &[TaggedItem],
            ChangeSource::Descriptor(0),
        ))
        .if_exists((
            ConstraintFactory::<TaggedItems, SoftScore>::new()
                .for_each(source(
                    tagged_items as fn(&TaggedItems) -> &[TaggedItem],
                    ChangeSource::Descriptor(0),
                ))
                .filter(|item: &TaggedItem| item.enabled),
            equal_bi(|item: &TaggedItem| item.key, |item: &TaggedItem| item.key),
        ))
        .penalize(SoftScore::of(1))
        .named("key has enabled peer");

    let mut state = TaggedItems {
        items: vec![
            TaggedItem {
                key: 1,
                enabled: false,
            },
            TaggedItem {
                key: 1,
                enabled: true,
            },
        ],
    };

    let mut total = constraint.initialize(&state);
    assert_eq!(total, SoftScore::of(-2));

    total = total + constraint.on_retract(&state, 1, 0);
    state.items[1].enabled = false;
    total = total + constraint.on_insert(&state, 1, 0);

    assert_eq!(total, constraint.evaluate(&state));
    assert_eq!(total, SoftScore::of(0));
}
