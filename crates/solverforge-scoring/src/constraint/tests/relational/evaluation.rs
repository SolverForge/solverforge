use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, JoinNode, Operator},
    Leaf,
};
use std::sync::atomic::{AtomicUsize, Ordering};
struct Model(Vec<u32>);
fn values(m: &Model) -> &[u32] {
    &m.0
}
fn key(v: &Leaf<'_, u32>) -> u32 {
    *v.entity
}
static CALLS: AtomicUsize = AtomicUsize::new(0);
fn counted_key(v: &Leaf<'_, u32>) -> u32 {
    CALLS.fetch_add(1, Ordering::Relaxed);
    *v.entity
}

#[derive(PartialEq, Eq, Hash)]
struct CountedKey(u32);
static CLONES: AtomicUsize = AtomicUsize::new(0);
impl Clone for CountedKey {
    fn clone(&self) -> Self {
        CLONES.fetch_add(1, Ordering::Relaxed);
        Self(self.0)
    }
}
#[test]
fn equality_probes_borrow_the_retained_bucket() {
    use crate::stream::joiner::plan::{ExecutablePlan, IndexedPlan};
    use crate::stream::relational::DenseRowStore;
    let plan = equal_bi(|v: &u32| *v, |v: &u32| *v);
    let mut indexes = plan.new_indexes();
    let mut store = DenseRowStore::new();
    let handle = store.insert(1);
    plan.insert_right(&mut indexes, handle, &1);
    let bucket = indexes.1.lookup(&1);
    let candidates = plan.right_candidates(&indexes, &1);
    assert_eq!(
        candidates.as_ptr(),
        bucket.as_ptr(),
        "equality probe must not allocate a bucket copy"
    );
}

#[test]
fn transient_evaluation_does_not_retain_reverse_keys() {
    let leaf = |binding| {
        CollectionNode::new(
            source(values as fn(&Model) -> &[u32], ChangeSource::Descriptor(0)),
            binding,
        )
    };
    let tree = JoinNode::new(
        leaf(0),
        leaf(1),
        equal_bi(
            |v: &Leaf<'_, u32>| CountedKey(*v.entity),
            |v: &Leaf<'_, u32>| CountedKey(*v.entity),
        ),
    );
    CLONES.store(0, Ordering::Relaxed);
    let mut rows = 0;
    tree.visit_all(&Model((0..64).collect()), &mut |_| rows += 1);
    assert_eq!(rows, 64);
    assert_eq!(
        CLONES.load(Ordering::Relaxed),
        0,
        "transient rows never retract"
    );
}

#[test]
fn full_evaluation_consumes_compiled_index_instead_of_cartesian_scan() {
    let leaf = |binding| {
        CollectionNode::new(
            source(values as fn(&Model) -> &[u32], ChangeSource::Descriptor(0)),
            binding,
        )
    };
    let tree = JoinNode::new(leaf(0), leaf(1), equal_bi(key, counted_key));
    let m = Model((0..64).collect());
    CALLS.store(0, Ordering::Relaxed);
    let mut rows = 0;
    tree.visit_all(&m, &mut |_| rows += 1);
    assert_eq!(rows, 64);
    assert!(
        CALLS.load(Ordering::Relaxed) == 64,
        "full evaluation must map right keys once, not repeat indexed equality checks"
    );
}
