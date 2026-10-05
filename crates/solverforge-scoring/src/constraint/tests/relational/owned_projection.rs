use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, JoinNode, Pair, ProjectNode, ProjectView},
    Leaf,
};
use solverforge_core::{score::SoftScore, ConstraintRef, ImpactType};

#[derive(Clone, Debug)]
struct Value {
    key: u32,
    weight: i64,
}
// Intentionally neither Clone, Copy, PartialEq nor Debug.
struct Payload {
    key: u32,
    weight: i64,
}
struct Model {
    values: Vec<Value>,
}
fn values(m: &Model) -> &[Value] {
    &m.values
}
fn project(_: &Model, row: &Leaf<'_, Value>) -> Vec<Payload> {
    (0..row.entity.weight.clamp(0, 2))
        .map(|_| Payload {
            key: row.entity.key,
            weight: row.entity.weight,
        })
        .collect()
}
fn projected_key(row: &ProjectView<'_, Leaf<'_, Value>, Payload>) -> u32 {
    row.value.key
}
fn key(row: &Leaf<'_, Value>) -> u32 {
    row.entity.key
}
fn weight(
    _: &Model,
    row: &Pair<Leaf<'_, Value>, ProjectView<'_, Leaf<'_, Value>, Payload>>,
) -> SoftScore {
    SoftScore::of(row.left.entity.weight + row.right.value.weight)
}
fn oracle(m: &Model) -> (SoftScore, usize) {
    let mut score = 0;
    let mut count = 0;
    for a in &m.values {
        for b in &m.values {
            if a.key == b.key {
                for _ in 0..b.weight.clamp(0, 2) {
                    score -= a.weight + b.weight;
                    count += 1;
                }
            }
        }
    }
    (SoftScore::of(score), count)
}
#[test]
fn owned_projection_recomputes_fresh_values_and_preserves_all_contributors() {
    let leaf = |binding| {
        CollectionNode::new(
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let projected = ProjectNode::new(leaf(1), project);
    let tree = JoinNode::new(leaf(0), projected, equal_bi(key, projected_key));
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "owned"),
        ImpactType::Penalty,
        tree,
        weight,
        false,
    );
    let mut m = Model {
        values: vec![Value { key: 1, weight: 2 }, Value { key: 1, weight: 1 }],
    };
    assert_eq!(c.evaluate(&m), oracle(&m).0);
    assert_eq!(c.match_count(&m), oracle(&m).1);
    assert!(c
        .get_matches(&m)
        .iter()
        .all(|x| x.justification.entities.len() == 2));
    let mut score = c.initialize(&m);
    for (index, new_key, new_weight) in [(0, 2, 9), (1, 2, 0), (0, 2, 1), (1, 2, 3)] {
        score = score + c.on_retract(&m, index, 0);
        assert_eq!(c.on_retract(&m, index, 0), SoftScore::of(0));
        m.values[index] = Value {
            key: new_key,
            weight: new_weight,
        };
        // Full evaluation cannot read stale retained projections between callbacks.
        assert_eq!(c.evaluate(&m), oracle(&m).0);
        score = score + c.on_insert(&m, index, 0);
        assert_eq!(c.on_insert(&m, index, 0), SoftScore::of(0));
        assert_eq!(score, oracle(&m).0);
        assert_eq!(c.match_count(&m), oracle(&m).1);
        let matches = c.get_matches(&m);
        assert_eq!(matches.len(), oracle(&m).1);
        assert_eq!(
            matches.iter().fold(SoftScore::of(0), |s, x| s + x.score),
            score
        );
        assert!(matches.iter().all(|x| x.justification.entities.len() == 2));
    }
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}

#[test]
fn nested_projection_retains_three_bindings_and_maps_each_input_once() {
    use crate::stream::relational::operator::Operator;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    CALLS.store(0, Ordering::Relaxed);
    let leaf = |binding| {
        CollectionNode::new(
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let upstream = JoinNode::new(leaf(0), leaf(1), equal_bi(key, key));
    fn emit(_: &Model, row: &Pair<Leaf<'_, Value>, Leaf<'_, Value>>) -> Vec<Payload> {
        CALLS.fetch_add(1, Ordering::Relaxed);
        vec![Payload {
            key: row.left.entity.key,
            weight: row.left.entity.weight + row.right.entity.weight,
        }]
    }
    fn projected(row: &ProjectView<'_, Pair<Leaf<'_, Value>, Leaf<'_, Value>>, Payload>) -> u32 {
        row.value.key
    }
    let tree = JoinNode::new(
        ProjectNode::new(upstream, emit),
        leaf(2),
        equal_bi(projected, key),
    );
    let m = Model {
        values: vec![Value { key: 1, weight: 2 }, Value { key: 1, weight: 3 }],
    };
    let mut count = 0;
    tree.visit_all(&m, &mut |_| count += 1);
    assert_eq!(count, 8);
    assert_eq!(CALLS.load(Ordering::Relaxed), 4);
    let mut tree = tree;
    tree.initialize(&m);
    for h in tree.handles() {
        let mut provenance = Vec::new();
        tree.visit_provenance(h, &mut |binding, descriptor, index| {
            provenance.push((binding, descriptor, index))
        });
        assert_eq!(provenance.len(), 3);
        assert_eq!(
            provenance.iter().map(|p| p.0).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }
}
