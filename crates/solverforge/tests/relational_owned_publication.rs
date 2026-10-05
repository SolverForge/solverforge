use solverforge::stream::{
    joiner::equal_bi,
    relational::{
        operator::{CollectionNode, JoinNode, Pair, ProjectNode, ProjectView},
        Leaf,
    },
    source, ChangeSource,
};
use solverforge::{IncrementalConstraint, SoftScore};
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::constraint::relational::OperatorTerminal;

#[derive(Clone, Debug)]
struct Entity {
    key: u32,
    weight: i64,
}
struct Payload {
    key: u32,
    weight: i64,
}
struct Model {
    entities: Vec<Entity>,
}
fn entities(m: &Model) -> &[Entity] {
    &m.entities
}
fn leaf_key(row: &Leaf<'_, Entity>) -> u32 {
    row.entity.key
}
fn emitted_key(row: &ProjectView<'_, Leaf<'_, Entity>, Payload>) -> u32 {
    row.value.key
}
fn emitted(_: &Model, row: &Leaf<'_, Entity>) -> Vec<Payload> {
    vec![Payload {
        key: row.entity.key,
        weight: row.entity.weight,
    }]
}
fn weight(
    _: &Model,
    row: &Pair<ProjectView<'_, Leaf<'_, Entity>, Payload>, Leaf<'_, Entity>>,
) -> SoftScore {
    SoftScore::of(row.left.value.weight + row.right.entity.weight)
}
#[test]
fn facade_projection_can_be_the_indexed_left_input_without_clone_payloads() {
    let leaf = |binding| {
        CollectionNode::new(
            source(
                entities as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let tree = JoinNode::new(
        ProjectNode::new(leaf(0), emitted),
        leaf(1),
        equal_bi(emitted_key, leaf_key),
    );
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("public", "owned"),
        ImpactType::Reward,
        tree,
        weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1, weight: 2 }, Entity { key: 1, weight: 3 }],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(20));
    assert_eq!(c.match_count(&m), 4);
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 0, 0);
    m.entities[0] = Entity { key: 2, weight: 9 };
    assert_eq!(c.evaluate(&m), SoftScore::of(24));
    score = score + c.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(24));
    assert_eq!(c.get_matches(&m).len(), 2);
    c.reset();
    assert_eq!(c.evaluate(&m), score);
}
