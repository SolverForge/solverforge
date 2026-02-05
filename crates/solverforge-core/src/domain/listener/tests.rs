//! Tests for variable listener infrastructure

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct TestSolution {
    value: i32,
}

struct TestEntity {
    id: usize,
}

struct CountingListener {
    before_count: Arc<AtomicUsize>,
    after_count: Arc<AtomicUsize>,
}

impl VariableListener<TestSolution, TestEntity> for CountingListener {
    fn before_variable_changed(&mut self, _solution: &TestSolution, _entity: &TestEntity) {
        self.before_count.fetch_add(1, Ordering::SeqCst);
    }

    fn after_variable_changed(&mut self, _solution: &mut TestSolution, _entity: &TestEntity) {
        self.after_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn test_variable_listener_calls() {
    let before_count = Arc::new(AtomicUsize::new(0));
    let after_count = Arc::new(AtomicUsize::new(0));

    let mut listener = CountingListener {
        before_count: before_count.clone(),
        after_count: after_count.clone(),
    };

    let mut solution = TestSolution { value: 0 };
    assert_eq!(solution.value, 0);
    let entity = TestEntity { id: 1 };
    assert_eq!(entity.id, 1);

    listener.before_variable_changed(&solution, &entity);
    listener.after_variable_changed(&mut solution, &entity);

    assert_eq!(before_count.load(Ordering::SeqCst), 1);
    assert_eq!(after_count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_requires_unique_entity_events_default() {
    let listener = CountingListener {
        before_count: Arc::new(AtomicUsize::new(0)),
        after_count: Arc::new(AtomicUsize::new(0)),
    };

    assert!(!listener.requires_unique_entity_events());
}
