use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, FlattenNode, FlattenView, JoinNode, Pair},
    Leaf,
};
use solverforge_core::{score::SoftScore, ConstraintRef, ImpactType};
#[derive(Clone, Debug)]
struct Entity {
    key: u32,
    children: Vec<i64>,
}
struct Model(Vec<Entity>);
fn entities(m: &Model) -> &[Entity] {
    &m.0
}
fn key(row: &Leaf<'_, Entity>) -> u32 {
    row.entity.key
}
fn children<'a>(_: &'a Model, row: Pair<Leaf<'a, Entity>, Leaf<'a, Entity>>) -> &'a [i64] {
    &row.right.entity.children
}
fn weight(
    _: &Model,
    row: &FlattenView<'_, Pair<Leaf<'_, Entity>, Leaf<'_, Entity>>, i64>,
) -> SoftScore {
    SoftScore::of(*row.value)
}
#[test]
fn borrowed_flattening_preserves_child_duplicates_and_both_owner_bindings() {
    let leaf = |binding| {
        CollectionNode::new(
            source(
                entities as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let joined = JoinNode::new(leaf(0), leaf(1), equal_bi(key, key));
    let flattened = FlattenNode::new(joined, children);
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "flatten"),
        ImpactType::Reward,
        flattened,
        weight,
        false,
    );
    let mut m = Model(vec![
        Entity {
            key: 1,
            children: vec![2, 2],
        },
        Entity {
            key: 1,
            children: vec![3],
        },
    ]);
    assert_eq!(c.evaluate(&m), SoftScore::of(14));
    assert_eq!(c.match_count(&m), 6);
    assert!(c
        .get_matches(&m)
        .iter()
        .all(|x| x.justification.entities.len() == 2));
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 0, 0);
    m.0[0].children = vec![9, 0, 9];
    m.0[0].key = 2;
    score = score + c.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(21));
    assert_eq!(score, c.evaluate(&m));
    assert_eq!(c.match_count(&m), 4);
    score = score + c.on_retract(&m, 1, 0);
    m.0[1].children.clear();
    score = score + c.on_insert(&m, 1, 0);
    assert_eq!(score, SoftScore::of(18));
    assert_eq!(c.match_count(&m), 3);
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}
