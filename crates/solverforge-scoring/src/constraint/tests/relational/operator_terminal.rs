use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, JoinNode, Pair},
    Leaf,
};
use solverforge_core::{score::SoftScore, ConstraintRef, ImpactType};

#[derive(Clone, Debug)]
struct Value {
    key: u32,
    weight: i64,
}
struct Model {
    values: Vec<Value>,
}
fn values(m: &Model) -> &[Value] {
    &m.values
}
fn key(v: &Leaf<'_, Value>) -> u32 {
    v.entity.key
}
fn weight(_: &Model, row: &Pair<Leaf<'_, Value>, Leaf<'_, Value>>) -> SoftScore {
    SoftScore::of(row.left.entity.weight + row.right.entity.weight)
}

#[test]
fn generic_terminal_retracts_retained_scores_and_explains_repeated_bindings() {
    let leaf = |binding| {
        CollectionNode::new(
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let tree = JoinNode::new(leaf(0), leaf(1), equal_bi(key, key));
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "tree"),
        ImpactType::Penalty,
        tree,
        weight,
        false,
    );
    let mut m = Model {
        values: vec![Value { key: 1, weight: 2 }, Value { key: 1, weight: 3 }],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(-20));
    assert_eq!(c.match_count(&m), 4);
    assert_eq!(c.get_matches(&m).len(), 4);
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 0, 0);
    m.values[0].key = 2;
    m.values[0].weight = 9;
    score = score + c.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(-24));
    assert_eq!(score, c.evaluate(&m));
    assert_eq!(c.match_count(&m), 2);
    let matches = c.get_matches(&m);
    assert!(matches.iter().all(|m| m.justification.entities.len() == 2));
    assert_eq!(
        matches.iter().fold(SoftScore::of(0), |s, m| s + m.score),
        score
    );
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}
