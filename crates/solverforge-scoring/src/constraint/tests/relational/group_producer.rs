use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::{collect_vec, Accumulator, CollectedVec};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, GroupNode, GroupView, JoinNode, Operator, Pair},
    Leaf,
};
use solverforge_core::{score::SoftScore, ConstraintRef, ImpactType};

#[derive(Clone, Debug)]
struct Entity {
    key: u32,
    value: i64,
}
struct Payload(i64);
struct Model {
    entities: Vec<Entity>,
    targets: Vec<usize>,
}
fn entities(m: &Model) -> &[Entity] {
    &m.entities
}
fn targets(m: &Model) -> &[usize] {
    &m.targets
}
fn key(v: &Leaf<'_, Entity>) -> u32 {
    v.entity.key
}
fn mapped(v: Leaf<'_, Entity>) -> Payload {
    Payload(v.entity.value)
}
fn aggregate_key<O: Operator<Model>, A: Accumulator<Payload, CollectedVec<Payload>>>(
    v: &GroupView<'_, Model, O, u32, A, Payload, CollectedVec<Payload>>,
) -> usize {
    v.with_result(|r| r.len())
}
fn target_key(v: &Leaf<'_, usize>) -> usize {
    *v.entity
}
fn weight<O: Operator<Model>, A: Accumulator<Payload, CollectedVec<Payload>>>(
    _: &Model,
    v: &Pair<GroupView<'_, Model, O, u32, A, Payload, CollectedVec<Payload>>, Leaf<'_, usize>>,
) -> SoftScore {
    v.left
        .with_result(|r| SoftScore::of(r.iter().map(|p| p.0).sum()))
}
#[test]
fn grouped_producer_retracts_exact_tokens_and_rekeys_downstream_on_both_callbacks() {
    let leaf = CollectionNode::new(
        source(
            entities as fn(&Model) -> &[Entity],
            ChangeSource::Descriptor(0),
        ),
        0,
    );
    let right = CollectionNode::new(
        source(
            targets as fn(&Model) -> &[usize],
            ChangeSource::Descriptor(1),
        ),
        1,
    );
    let group = GroupNode::new(leaf, key, collect_vec(mapped));
    let tree = JoinNode::new(group, right, equal_bi(aggregate_key, target_key));
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "group"),
        ImpactType::Reward,
        tree,
        weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1, value: 2 }, Entity { key: 1, value: 3 }],
        targets: vec![1, 2],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(5));
    assert_eq!(c.get_matches(&m)[0].justification.entities.len(), 3);
    let mut score = c.initialize(&m);
    // Removing one contributor leaves a changed group, not an absent group.
    score = score + c.on_retract(&m, 0, 0);
    assert_eq!(score, SoftScore::of(3));
    assert_eq!(c.on_retract(&m, 0, 0), SoftScore::of(0));
    m.entities[0] = Entity { key: 2, value: 9 };
    assert_eq!(c.evaluate(&m), SoftScore::of(12));
    score = score + c.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(12));
    assert_eq!(c.on_insert(&m, 0, 0), SoftScore::of(0));
    assert_eq!(c.get_matches(&m).len(), 2);
    assert!(c
        .get_matches(&m)
        .iter()
        .all(|x| x.justification.entities.len() == 2));
    score = score + c.on_retract(&m, 1, 0);
    assert_eq!(score, SoftScore::of(9));
    m.entities[1] = Entity { key: 2, value: 4 };
    score = score + c.on_insert(&m, 1, 0);
    assert_eq!(score, SoftScore::of(13));
    assert_eq!(score, c.evaluate(&m));
    assert_eq!(c.get_matches(&m)[0].justification.entities.len(), 3);
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}

fn pair_key(row: &Pair<Leaf<'_, Entity>, Leaf<'_, Entity>>) -> u32 {
    row.left.entity.key
}
fn mapped_pair(row: Pair<Leaf<'_, Entity>, Leaf<'_, Entity>>) -> Payload {
    Payload(row.left.entity.value + row.right.entity.value)
}
fn group_key<O: Operator<Model>, A: Accumulator<Payload, CollectedVec<Payload>>>(
    row: &GroupView<'_, Model, O, u32, A, Payload, CollectedVec<Payload>>,
) -> u32 {
    *row.key
}
fn right_group_weight<O: Operator<Model>, A: Accumulator<Payload, CollectedVec<Payload>>>(
    _: &Model,
    row: &Pair<Leaf<'_, Entity>, GroupView<'_, Model, O, u32, A, Payload, CollectedVec<Payload>>>,
) -> SoftScore {
    row.right
        .with_result(|r| SoftScore::of(r.iter().map(|p| p.0).sum()))
}
#[test]
fn joined_group_as_right_target_coalesces_repeated_descriptor_changes_against_oracle() {
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
    let grouped = GroupNode::new(joined, pair_key, collect_vec(mapped_pair));
    let tree = JoinNode::new(leaf(2), grouped, equal_bi(key, group_key));
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "joined group"),
        ImpactType::Reward,
        tree,
        right_group_weight,
        false,
    );
    let mut m = Model {
        entities: vec![
            Entity { key: 1, value: 2 },
            Entity { key: 1, value: 3 },
            Entity { key: 2, value: 4 },
        ],
        targets: vec![],
    };
    let oracle = |m: &Model| {
        let mut sum = 0;
        for a in &m.entities {
            for b in &m.entities {
                for d in &m.entities {
                    if a.key == b.key && b.key == d.key {
                        sum += b.value + d.value;
                    }
                }
            }
        }
        SoftScore::of(sum)
    };
    assert_eq!(c.evaluate(&m), oracle(&m));
    assert_eq!(c.get_matches(&m).len(), 3);
    let mut score = c.initialize(&m);
    for step in 0..90 {
        let i = step % 3;
        score = score + c.on_retract(&m, i, 0);
        m.entities[i].key = (step % 4) as u32;
        m.entities[i].value = step as i64 - 20;
        score = score + c.on_insert(&m, i, 0);
        assert_eq!(score, oracle(&m));
        assert_eq!(c.evaluate(&m), score);
        assert_eq!(c.match_count(&m), 3);
        let matches = c.get_matches(&m);
        assert_eq!(
            matches
                .iter()
                .fold(SoftScore::of(0), |sum, m| sum + m.score),
            score
        );
        for matched in matches {
            assert!(matched.justification.entities.len() >= 3);
        }
    }
}
