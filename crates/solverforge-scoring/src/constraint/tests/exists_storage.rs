use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::exists::ExistsStorageKind;
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

#[derive(Clone)]
struct CustomerState {
    customers: Vec<usize>,
}

fn customers(state: &CustomerState) -> &[usize] {
    state.customers.as_slice()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key(usize);

#[derive(Clone)]
struct KeyValues {
    values: Vec<Key>,
}

fn key_values(state: &KeyValues) -> &[Key] {
    state.values.as_slice()
}

#[test]
fn test_exists_storage_selects_indexed_only_for_exact_usize_keys() {
    let usize_constraint = ConstraintFactory::<CustomerState, SoftScore>::new()
        .for_each(source(
            customers as fn(&CustomerState) -> &[usize],
            ChangeSource::Static,
        ))
        .if_exists((
            ConstraintFactory::<CustomerState, SoftScore>::new().for_each(source(
                customers as fn(&CustomerState) -> &[usize],
                ChangeSource::Static,
            )),
            equal_bi(|left: &usize| *left, |right: &usize| *right),
        ))
        .penalize(SoftScore::of(1))
        .named("usize exists");
    assert_eq!(
        usize_constraint.storage_kind(),
        ExistsStorageKind::IndexedUsize
    );

    let option_constraint = ConstraintFactory::<TaskSchedule, SoftScore>::new()
        .for_each(source(
            tasks as fn(&TaskSchedule) -> &[Task],
            ChangeSource::Static,
        ))
        .if_exists((
            ConstraintFactory::<TaskSchedule, SoftScore>::new().for_each(source(
                workers as fn(&TaskSchedule) -> &[Worker],
                ChangeSource::Static,
            )),
            equal_bi(
                |task: &Task| task.assignee,
                |worker: &Worker| Some(worker.id),
            ),
        ))
        .penalize(SoftScore::of(1))
        .named("option exists");
    assert_eq!(option_constraint.storage_kind(), ExistsStorageKind::Hashed);

    let newtype_constraint = ConstraintFactory::<KeyValues, SoftScore>::new()
        .for_each(source(
            key_values as fn(&KeyValues) -> &[Key],
            ChangeSource::Static,
        ))
        .if_exists((
            ConstraintFactory::<KeyValues, SoftScore>::new().for_each(source(
                key_values as fn(&KeyValues) -> &[Key],
                ChangeSource::Static,
            )),
            equal_bi(|left: &Key| *left, |right: &Key| *right),
        ))
        .penalize(SoftScore::of(1))
        .named("newtype exists");
    assert_eq!(newtype_constraint.storage_kind(), ExistsStorageKind::Hashed);
}

#[derive(Clone)]
struct DirectTask<K> {
    key: K,
}

#[derive(Clone)]
struct DirectAssignment<K> {
    key: K,
}

#[derive(Clone)]
struct DirectState<K> {
    tasks: Vec<DirectTask<K>>,
    assignments: Vec<DirectAssignment<K>>,
}

fn direct_tasks<K>(state: &DirectState<K>) -> &[DirectTask<K>] {
    state.tasks.as_slice()
}

fn direct_assignments<K>(state: &DirectState<K>) -> &[DirectAssignment<K>] {
    state.assignments.as_slice()
}

#[test]
fn test_direct_if_exists_indexed_and_hashed_newtype_parity() {
    let mut usize_constraint = ConstraintFactory::<DirectState<usize>, SoftScore>::new()
        .for_each(source(
            direct_tasks::<usize> as fn(&DirectState<usize>) -> &[DirectTask<usize>],
            ChangeSource::Static,
        ))
        .if_exists((
            ConstraintFactory::<DirectState<usize>, SoftScore>::new().for_each(source(
                direct_assignments::<usize>
                    as fn(&DirectState<usize>) -> &[DirectAssignment<usize>],
                ChangeSource::Descriptor(0),
            )),
            equal_bi(
                |task: &DirectTask<usize>| task.key,
                |assignment: &DirectAssignment<usize>| assignment.key,
            ),
        ))
        .penalize(SoftScore::of(1))
        .named("direct usize exists");
    let mut key_constraint = ConstraintFactory::<DirectState<Key>, SoftScore>::new()
        .for_each(source(
            direct_tasks::<Key> as fn(&DirectState<Key>) -> &[DirectTask<Key>],
            ChangeSource::Static,
        ))
        .if_exists((
            ConstraintFactory::<DirectState<Key>, SoftScore>::new().for_each(source(
                direct_assignments::<Key> as fn(&DirectState<Key>) -> &[DirectAssignment<Key>],
                ChangeSource::Descriptor(0),
            )),
            equal_bi(
                |task: &DirectTask<Key>| task.key,
                |assignment: &DirectAssignment<Key>| assignment.key,
            ),
        ))
        .penalize(SoftScore::of(1))
        .named("direct newtype exists");

    assert_eq!(
        usize_constraint.storage_kind(),
        ExistsStorageKind::IndexedUsize
    );
    assert_eq!(key_constraint.storage_kind(), ExistsStorageKind::Hashed);

    let mut usize_state = DirectState {
        tasks: vec![
            DirectTask { key: 0 },
            DirectTask { key: 1 },
            DirectTask { key: 2 },
            DirectTask { key: 3 },
        ],
        assignments: vec![DirectAssignment { key: 0 }, DirectAssignment { key: 2 }],
    };
    let mut key_state = DirectState {
        tasks: vec![
            DirectTask { key: Key(0) },
            DirectTask { key: Key(1) },
            DirectTask { key: Key(2) },
            DirectTask { key: Key(3) },
        ],
        assignments: vec![
            DirectAssignment { key: Key(0) },
            DirectAssignment { key: Key(2) },
        ],
    };

    let mut usize_total = usize_constraint.initialize(&usize_state);
    let mut key_total = key_constraint.initialize(&key_state);
    assert_eq!(usize_total, key_total);
    assert_eq!(usize_total, SoftScore::of(-2));
    assert_eq!(
        usize_constraint.match_count(&usize_state),
        key_constraint.match_count(&key_state)
    );

    usize_total = usize_total + usize_constraint.on_retract(&usize_state, 0, 0);
    key_total = key_total + key_constraint.on_retract(&key_state, 0, 0);
    usize_state.assignments[0].key = 3;
    key_state.assignments[0].key = Key(3);
    usize_total = usize_total + usize_constraint.on_insert(&usize_state, 0, 0);
    key_total = key_total + key_constraint.on_insert(&key_state, 0, 0);

    assert_eq!(usize_total, key_total);
    assert_eq!(usize_total, usize_constraint.evaluate(&usize_state));
    assert_eq!(key_total, key_constraint.evaluate(&key_state));
    assert_eq!(usize_constraint.match_count(&usize_state), 2);
    assert_eq!(
        usize_constraint.match_count(&usize_state),
        key_constraint.match_count(&key_state)
    );
}

#[derive(Clone)]
struct FlattenedState<K> {
    customers: Vec<K>,
    routes: Vec<Vec<K>>,
}

fn flattened_customers<K>(state: &FlattenedState<K>) -> &[K] {
    state.customers.as_slice()
}

fn flattened_routes<K>(state: &FlattenedState<K>) -> &[Vec<K>] {
    state.routes.as_slice()
}

#[test]
fn test_flattened_if_not_exists_indexed_and_hashed_newtype_parity() {
    let mut usize_constraint = ConstraintFactory::<FlattenedState<usize>, SoftScore>::new()
        .for_each(source(
            flattened_customers::<usize> as fn(&FlattenedState<usize>) -> &[usize],
            ChangeSource::Static,
        ))
        .if_not_exists((
            ConstraintFactory::<FlattenedState<usize>, SoftScore>::new()
                .for_each(source(
                    flattened_routes::<usize> as fn(&FlattenedState<usize>) -> &[Vec<usize>],
                    ChangeSource::Descriptor(0),
                ))
                .flattened(|route: &Vec<usize>| route),
            equal_bi(|customer: &usize| *customer, |assigned: &usize| *assigned),
        ))
        .penalize(SoftScore::of(1))
        .named("flattened usize missing assignment");
    let mut key_constraint = ConstraintFactory::<FlattenedState<Key>, SoftScore>::new()
        .for_each(source(
            flattened_customers::<Key> as fn(&FlattenedState<Key>) -> &[Key],
            ChangeSource::Static,
        ))
        .if_not_exists((
            ConstraintFactory::<FlattenedState<Key>, SoftScore>::new()
                .for_each(source(
                    flattened_routes::<Key> as fn(&FlattenedState<Key>) -> &[Vec<Key>],
                    ChangeSource::Descriptor(0),
                ))
                .flattened(|route: &Vec<Key>| route),
            equal_bi(|customer: &Key| *customer, |assigned: &Key| *assigned),
        ))
        .penalize(SoftScore::of(1))
        .named("flattened newtype missing assignment");

    assert_eq!(
        usize_constraint.storage_kind(),
        ExistsStorageKind::IndexedUsize
    );
    assert_eq!(key_constraint.storage_kind(), ExistsStorageKind::Hashed);

    let mut usize_state = FlattenedState {
        customers: vec![0, 1, 2, 3, 4],
        routes: vec![vec![0, 1], vec![3]],
    };
    let mut key_state = FlattenedState {
        customers: vec![Key(0), Key(1), Key(2), Key(3), Key(4)],
        routes: vec![vec![Key(0), Key(1)], vec![Key(3)]],
    };

    let mut usize_total = usize_constraint.initialize(&usize_state);
    let mut key_total = key_constraint.initialize(&key_state);
    assert_eq!(usize_total, key_total);
    assert_eq!(usize_total, SoftScore::of(-2));

    usize_total = usize_total + usize_constraint.on_retract(&usize_state, 0, 0);
    key_total = key_total + key_constraint.on_retract(&key_state, 0, 0);
    usize_state.routes[0] = vec![2, 4];
    key_state.routes[0] = vec![Key(2), Key(4)];
    usize_total = usize_total + usize_constraint.on_insert(&usize_state, 0, 0);
    key_total = key_total + key_constraint.on_insert(&key_state, 0, 0);

    assert_eq!(usize_total, key_total);
    assert_eq!(usize_total, usize_constraint.evaluate(&usize_state));
    assert_eq!(key_total, key_constraint.evaluate(&key_state));
    assert_eq!(
        usize_constraint.match_count(&usize_state),
        key_constraint.match_count(&key_state)
    );

    usize_total = usize_total + usize_constraint.on_retract(&usize_state, 1, 0);
    key_total = key_total + key_constraint.on_retract(&key_state, 1, 0);
    usize_state.routes[1] = vec![0, 3, 4];
    key_state.routes[1] = vec![Key(0), Key(3), Key(4)];
    usize_total = usize_total + usize_constraint.on_insert(&usize_state, 1, 0);
    key_total = key_total + key_constraint.on_insert(&key_state, 1, 0);

    assert_eq!(usize_total, key_total);
    assert_eq!(usize_total, usize_constraint.evaluate(&usize_state));
    assert_eq!(key_total, key_constraint.evaluate(&key_state));
    assert_eq!(
        usize_constraint.match_count(&usize_state),
        key_constraint.match_count(&key_state)
    );
}

#[test]
fn test_flattened_if_exists_uses_indexed_usize_storage() {
    let mut constraint = ConstraintFactory::<FlattenedState<usize>, SoftScore>::new()
        .for_each(source(
            flattened_customers::<usize> as fn(&FlattenedState<usize>) -> &[usize],
            ChangeSource::Static,
        ))
        .if_exists((
            ConstraintFactory::<FlattenedState<usize>, SoftScore>::new()
                .for_each(source(
                    flattened_routes::<usize> as fn(&FlattenedState<usize>) -> &[Vec<usize>],
                    ChangeSource::Descriptor(0),
                ))
                .flattened(|route: &Vec<usize>| route),
            equal_bi(|customer: &usize| *customer, |assigned: &usize| *assigned),
        ))
        .penalize(SoftScore::of(1))
        .named("flattened usize assigned");

    assert_eq!(constraint.storage_kind(), ExistsStorageKind::IndexedUsize);

    let mut state = FlattenedState {
        customers: vec![0, 1, 2, 3, 4],
        routes: vec![vec![0, 2], Vec::new()],
    };

    let mut total = constraint.initialize(&state);
    assert_eq!(total, SoftScore::of(-2));
    assert_eq!(constraint.match_count(&state), 2);

    total = total + constraint.on_retract(&state, 1, 0);
    state.routes[1] = vec![1, 3, 4];
    total = total + constraint.on_insert(&state, 1, 0);

    assert_eq!(total, constraint.evaluate(&state));
    assert_eq!(total, SoftScore::of(-5));
    assert_eq!(constraint.match_count(&state), 5);
}
