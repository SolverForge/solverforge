/* Relational primitive contracts: borrowed rows, provenance, delta coalescing.

Pins the `stream::relational` behavior the operators build on: leaf rows
borrow without entity cloning or solution borrows, provenance keeps every
authored binding in tuple orientation, and delta epochs coalesce
duplicate paths with retract-before-insert ordering. Recursive row
concatenation and physical dependency traversal arrive with the chaining
operator and root update router that consume them.
*/

#[test]
fn borrowed_leaf_rows_carry_entities_and_semantic_indexes() {
    use crate::stream::relational::Leaf;

    #[derive(Debug, PartialEq)]
    struct Assignment {
        shift_id: u32,
    }

    let assignment = Assignment { shift_id: 10 };

    // Leaf rows borrow entities with semantic source indexes.
    let leaf = Leaf::new(&assignment, 2);
    assert_eq!(leaf.entity.shift_id, 10);
    assert_eq!(leaf.index, 2);

    // Rows are Copy over shared borrows: re-traversal costs nothing.
    let again = leaf;
    assert_eq!(again.entity.shift_id, 10);
}

#[test]
fn provenance_keeps_every_authored_binding_in_order() {
    use crate::stream::relational::{BindingId, Participation, Provenance};

    let mut store = crate::stream::relational::DenseRowStore::new();
    let row_a = store.insert("a");
    let row_b = store.insert("b");

    // One descriptor (0) in two bindings plus a second descriptor: three
    // authored participations, nothing deduplicated.
    let mut provenance = Provenance::new();
    provenance.push(Participation::new(BindingId(0), row_a, 0, 3));
    provenance.push(Participation::new(BindingId(1), row_a, 0, 3));
    provenance.push(Participation::new(BindingId(2), row_b, 1, 0));

    // Authored order is tuple orientation: the repeated entity appears
    // twice, once per binding.
    let bindings: Vec<u32> = provenance
        .bindings()
        .iter()
        .map(|part| part.binding.0)
        .collect();
    assert_eq!(bindings, vec![0, 1, 2]);

    // Handles participate by identity.
    assert_ne!(row_a, row_b);
}

#[test]
fn delta_epochs_coalesce_with_retract_before_insert_order() {
    use crate::stream::relational::{DeltaBuffer, DenseRowStore, JoinedIdentity, OutputDelta};

    let mut store = DenseRowStore::new();
    let left = store.insert("left");
    let right = store.insert("right");
    let first = JoinedIdentity::new(left, right, 0);
    let second = JoinedIdentity::new(left, right, 1);

    let mut buffer = DeltaBuffer::new();
    assert!(buffer.is_empty());
    buffer.begin();

    // Duplicate paths coalesce: the second push for `first` is dropped.
    assert!(buffer.push(OutputDelta::insert(first)));
    assert!(!buffer.push(OutputDelta::insert(first)));
    assert!(buffer.push(OutputDelta::retract(second)));

    // Retractions drain before insertions with deterministic row order.
    let drained = buffer.drain_ordered();
    assert_eq!(
        drained,
        vec![OutputDelta::retract(second), OutputDelta::insert(first)]
    );
    assert!(buffer.is_empty());

    // A new epoch republishes freely.
    buffer.begin();
    assert!(buffer.push(OutputDelta::insert(first)));
    assert!(!buffer.is_empty());
}
