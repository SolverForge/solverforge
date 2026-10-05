use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, ExistenceNode, Operator},
    Leaf,
};
use solverforge_core::{score::SoftScore, ConstraintRef, ImpactType};
#[derive(Clone, Debug)]
struct Entity {
    key: u32,
    weight: i64,
}
struct Model {
    left: Vec<Entity>,
    right: Vec<Entity>,
}
fn left(m: &Model) -> &[Entity] {
    &m.left
}
fn right(m: &Model) -> &[Entity] {
    &m.right
}
fn key(v: &Leaf<'_, Entity>) -> u32 {
    v.entity.key
}
fn weight(_: &Model, v: &Leaf<'_, Entity>) -> SoftScore {
    SoftScore::of(v.entity.weight)
}
#[test]
fn semi_and_anti_joins_publish_only_zero_crossings_and_preserve_left_identity() {
    for exists in [true, false] {
        let l = CollectionNode::new(
            source(left as fn(&Model) -> &[Entity], ChangeSource::Descriptor(0)),
            0,
        );
        let r = CollectionNode::new(
            source(
                right as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(1),
            ),
            1,
        );
        let mut tree = ExistenceNode::new(l, r, equal_bi(key, key), exists);
        let mut m = Model {
            left: vec![Entity { key: 1, weight: 5 }],
            right: vec![Entity { key: 1, weight: 1 }, Entity { key: 1, weight: 2 }],
        };
        tree.initialize(&m);
        let initial = tree.handles();
        assert!(tree.retract(&m, 1, 0).is_empty()); // 2 -> 1
        let delta = tree.retract(&m, 1, 1); // 1 -> 0
        assert_eq!(delta.len(), 1);
        if exists {
            assert_eq!(delta.removed, initial);
        } else {
            assert_eq!(delta.inserted, tree.handles());
        }
        let delta = tree.insert(&m, 1, 0); // 0 -> 1
        assert_eq!(delta.len(), 1);
        assert!(tree.insert(&m, 1, 1).is_empty()); // 1 -> 2
        assert_eq!(tree.handles(), initial);
        let mut c = OperatorTerminal::new(
            ConstraintRef::new("test", "exists"),
            ImpactType::Reward,
            tree,
            weight,
            false,
        );
        let mut score = c.initialize(&m);
        let oracle = |m: &Model| {
            SoftScore::of(
                m.left
                    .iter()
                    .filter(|l| m.right.iter().any(|r| l.key == r.key) == exists)
                    .map(|e| e.weight)
                    .sum(),
            )
        };
        for step in 0..90 {
            let descriptor = step % 2;
            let index = if descriptor == 0 { 0 } else { (step / 2) % 2 };
            score = score + c.on_retract(&m, index, descriptor);
            let values = if descriptor == 0 {
                &mut m.left
            } else {
                &mut m.right
            };
            values[index].key = (step % 3) as u32;
            values[index].weight = step as i64 - 20;
            score = score + c.on_insert(&m, index, descriptor);
            assert_eq!(score, oracle(&m));
            assert_eq!(c.evaluate(&m), score);
            assert_eq!(c.get_matches(&m).len(), c.match_count(&m));
        }
        c.reset();
        assert_eq!(c.initialize(&m), oracle(&m));
    }
}

#[test]
fn grouped_right_replacement_changes_semi_join_membership_during_retraction() {
    use crate::stream::collector::{count, CountAccumulator};
    use crate::stream::relational::operator::{GroupNode, GroupView};
    fn aggregate<O: Operator<Model>>(
        v: &GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
    ) -> u32 {
        v.with_result(|count| *count as u32)
    }
    let l = CollectionNode::new(
        source(left as fn(&Model) -> &[Entity], ChangeSource::Descriptor(0)),
        0,
    );
    let r = CollectionNode::new(
        source(
            right as fn(&Model) -> &[Entity],
            ChangeSource::Descriptor(1),
        ),
        1,
    );
    let grouped = GroupNode::new(r, key, count());
    let tree = ExistenceNode::new(l, grouped, equal_bi(key, aggregate), true);
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "grouped exists"),
        ImpactType::Reward,
        tree,
        weight,
        false,
    );
    let mut m = Model {
        left: vec![Entity { key: 1, weight: 5 }, Entity { key: 2, weight: 9 }],
        right: vec![Entity { key: 1, weight: 1 }, Entity { key: 1, weight: 2 }],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(9));
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 0, 1);
    assert_eq!(score, SoftScore::of(5));
    m.right[0].key = 2;
    assert_eq!(c.evaluate(&m), SoftScore::of(5));
    score = score + c.on_insert(&m, 0, 1);
    assert_eq!(score, SoftScore::of(5));
    assert_eq!(c.match_count(&m), 1);
    assert_eq!(c.get_matches(&m)[0].justification.entities.len(), 1);
}
