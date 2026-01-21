//! Variable listener infrastructure for shadow variable updates.
//!
//! Variable listeners are notified when genuine planning variables change,
//! allowing them to update shadow variables accordingly.
//!
//! # Architecture
//!
//! - [`VariableListener`]: Listens to basic/chained variable changes
//! - [`ListVariableListener`]: Listens to list variable changes with range info
//!
//! # Listener Types
//!
//! - **Automatic listeners**: Built-in listeners for Index, Inverse, Next/Previous
//! - **Custom listeners**: User-defined listeners for complex shadow variables

/// A listener that is notified when a source planning variable changes.
///
/// Variable listeners update shadow variables in response to genuine variable changes.
/// The listener is called with before/after notifications to allow capturing old state
/// if needed for correct shadow variable calculation.
///
/// # Implementation Notes
///
/// - Keep implementations stateless when possible
/// - Only modify shadow variables for which this listener is configured
/// - Never modify genuine variables or problem facts
/// - A single listener can update shadow variables on multiple entities
///   (e.g., changing arrival time affects all downstream entities in a route)
///
/// # Type Parameters
///
/// - `Solution`: The solution type (has `PlanningSolution` trait)
/// - `Entity`: The entity type on which the source variable is declared
pub trait VariableListener<Solution, Entity>: Send + Sync {
    /// Called before the source variable changes on the entity.
    ///
    /// Use this to capture any old state needed for shadow variable calculation.
    fn before_variable_changed(&mut self, solution: &Solution, entity: &Entity);

    /// Called after the source variable has changed on the entity.
    ///
    /// Update shadow variables based on the new variable value.
    fn after_variable_changed(&mut self, solution: &mut Solution, entity: &Entity);

    /// Whether this listener requires unique entity events.
    ///
    /// When `true`, each before/after method is guaranteed to be called only once
    /// per entity instance per operation (add, change, or remove).
    ///
    /// When `false` (default), the same entity may receive multiple notifications
    /// in a single operation, which can be more efficient but requires idempotent logic.
    fn requires_unique_entity_events(&self) -> bool {
        false
    }

    /// Called when the working solution changes.
    ///
    /// Override this to reset any internal state when a new solution is loaded.
    fn reset_working_solution(&mut self, _solution: &Solution) {}

    /// Called when the listener is no longer needed.
    ///
    /// Override this to clean up any resources.
    fn close(&mut self) {}
}

/// A listener that is notified when a list planning variable changes.
///
/// List variable listeners receive more detailed information about changes,
/// including the affected range within the list. This allows for efficient
/// incremental updates of shadow variables.
///
/// # Range Semantics
///
/// The `from_index` (inclusive) and `to_index` (exclusive) define the affected range.
/// - Before: The range about to change (may include elements being removed/moved)
/// - After: The range that changed (may include newly added elements)
///
/// Note that before and after ranges may differ if elements were added or removed.
///
/// # Type Parameters
///
/// - `Solution`: The solution type
/// - `Entity`: The entity type with the list variable
/// - `Element`: The type of elements in the list
pub trait ListVariableListener<Solution, Entity, Element>: Send + Sync {
    /// Called when an element is unassigned from any list.
    ///
    /// This is called during move undo or element removal.
    /// The listener should reset any shadow variables on the element to their
    /// unassigned state (typically `None` or default values).
    fn after_element_unassigned(&mut self, solution: &mut Solution, element: &Element);

    /// Called before elements in the range `[from_index, to_index)` change.
    ///
    /// Use this to capture any old state needed for shadow variable calculation.
    fn before_list_variable_changed(
        &mut self,
        solution: &Solution,
        entity: &Entity,
        from_index: usize,
        to_index: usize,
    );

    /// Called after elements in the range `[from_index, to_index)` have changed.
    ///
    /// Update shadow variables for elements within and potentially after the range.
    fn after_list_variable_changed(
        &mut self,
        solution: &mut Solution,
        entity: &Entity,
        from_index: usize,
        to_index: usize,
    );

    /// Called when the working solution changes.
    fn reset_working_solution(&mut self, _solution: &Solution) {}

    /// Called when the listener is no longer needed.
    fn close(&mut self) {}
}

/// Notification type for variable listener events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableNotification {
    /// Entity was added to the working solution.
    EntityAdded,
    /// Entity was removed from the working solution.
    EntityRemoved,
    /// A variable on the entity changed.
    VariableChanged,
}

/// Notification type for list variable listener events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListVariableNotification {
    /// An element was unassigned from all lists.
    ElementUnassigned,
    /// Elements in a range changed.
    RangeChanged {
        /// Inclusive start of affected range.
        from_index: usize,
        /// Exclusive end of affected range.
        to_index: usize,
    },
}

#[cfg(test)]
mod tests {
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
}
