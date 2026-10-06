/* Derived streams as right-hand join inputs (acceptance #6).

The common operator tree already joins *onto* grouped, projected, and
complemented producers (`group_producer.rs`, `owned_projection.rs`,
`complement_producer.rs`). This suite closes the two kinds that had no
right-hand coverage: a **filtered** producer and a **complemented** producer
used as the right input of an outer join, each scored against a hand-computed
oracle across descriptor churn.
*/

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource, SourceExtract};
use crate::stream::collector::{count, CountAccumulator};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{
        CollectionNode, ComplementNode, ComplementView, FilterNode, GroupNode, GroupView, JoinNode,
        Operator, Pair,
    },
    Leaf,
};
use solverforge_core::{score::SoftScore, ConstraintRef, ImpactType};

#[derive(Clone, Debug)]
struct Entity {
    key: u32,
}

struct Payload(i64);

struct Model {
    entities: Vec<Entity>,
    targets: Vec<u32>,
}

fn entities(m: &Model) -> &[Entity] {
    &m.entities
}
fn targets(m: &Model) -> &[u32] {
    &m.targets
}
fn key(row: &Leaf<'_, Entity>) -> u32 {
    row.entity.key
}
fn target_key(row: &Leaf<'_, u32>) -> u32 {
    *row.entity
}
fn group_key<O: Operator<Model>>(
    row: &GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
) -> u32 {
    *row.key
}

fn leaf(binding: u32) -> CollectionNode<Model, SourceExtract<fn(&Model) -> &[Entity]>> {
    CollectionNode::new(
        source(
            entities as fn(&Model) -> &[Entity],
            ChangeSource::Descriptor(0),
        ),
        binding,
    )
}

// FILTERED producer as the right input: only entities whose key is 1 survive
// the right-hand filter, and the outer join keys on equality.
fn filtered_weight(_: &Model, row: &Pair<Leaf<'_, Entity>, Leaf<'_, Entity>>) -> SoftScore {
    let _ = row.right.entity.key;
    SoftScore::of(1)
}

#[test]
fn filtered_producer_as_right_input_matches_oracle_across_churn() {
    let right = FilterNode::new(leaf(1), |_: &Model, v: &Leaf<'_, Entity>| v.entity.key == 1);
    let tree = JoinNode::new(leaf(0), right, equal_bi(key, key));
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "filtered right"),
        ImpactType::Reward,
        tree,
        filtered_weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1 }, Entity { key: 1 }, Entity { key: 2 }],
        targets: vec![],
    };
    let oracle = |m: &Model| {
        let mut matches = 0i64;
        for a in &m.entities {
            for b in &m.entities {
                if b.key == 1 && a.key == b.key {
                    matches += 1;
                }
            }
        }
        SoftScore::of(matches)
    };
    assert_eq!(c.evaluate(&m), oracle(&m));
    let mut score = c.initialize(&m);
    for step in 0..90 {
        let i = step % 3;
        score = score + c.on_retract(&m, i, 0);
        m.entities[i].key = (step % 3) as u32;
        score = score + c.on_insert(&m, i, 0);
        assert_eq!(score, oracle(&m));
        assert_eq!(c.evaluate(&m), score);
    }
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}

// COMPLEMENTED producer as the right input: each outer-join row pairs a
// target with its complement row (a real group or a default placeholder).
fn complement_default(_: &Model, _: &Leaf<'_, u32>) -> Payload {
    Payload(-10)
}
fn complemented_key<O: Operator<Model>>(
    row: &ComplementView<
        '_,
        Leaf<'_, u32>,
        GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
        Payload,
    >,
) -> u32 {
    match row {
        ComplementView::Real(pair) => *pair.left.entity,
        ComplementView::Default(projected) => *projected.input.entity,
    }
}

#[test]
fn complemented_producer_as_right_input_matches_oracle_across_churn() {
    fn joined_weight<O: Operator<Model>>(
        _: &Model,
        row: &Pair<
            Leaf<'_, u32>,
            ComplementView<
                '_,
                Leaf<'_, u32>,
                GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
                Payload,
            >,
        >,
    ) -> SoftScore {
        let _ = row.left.entity;
        // Recompute the complement contribution for exactly this right row.
        match &row.right {
            ComplementView::Real(pair) => pair.right.with_result(|r| SoftScore::of(*r as i64 * 5)),
            ComplementView::Default(projected) => SoftScore::of(projected.value.0),
        }
    }
    let target = |binding| {
        CollectionNode::new(
            source(targets as fn(&Model) -> &[u32], ChangeSource::Descriptor(1)),
            binding,
        )
    };
    let group = GroupNode::new(leaf(0), key, count());
    let complemented = ComplementNode::new(
        target(1),
        group,
        equal_bi(target_key, group_key),
        complement_default,
    );
    // The complemented producer is the RIGHT input, joined to a target leaf.
    let tree = JoinNode::new(
        target(2),
        complemented,
        equal_bi(target_key, complemented_key),
    );
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "complemented right"),
        ImpactType::Reward,
        tree,
        joined_weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1 }, Entity { key: 1 }],
        targets: vec![1, 1, 2],
    };
    let oracle = |m: &Model| {
        let mut total = 0i64;
        let mut matches = 0usize;
        for a in &m.targets {
            let count = m.entities.iter().filter(|e| e.key == *a).count();
            let per = if count == 0 { -10 } else { count as i64 * 5 };
            for b in &m.targets {
                if a == b {
                    total += per;
                    matches += 1;
                }
            }
        }
        (SoftScore::of(total), matches)
    };
    let mut score = c.initialize(&m);
    for step in 0..90 {
        let descriptor = step % 2;
        let index = (step / 2) % if descriptor == 0 { 2 } else { 3 };
        score = score + c.on_retract(&m, index, descriptor);
        if descriptor == 0 {
            m.entities[index].key = (step % 4) as u32;
        } else {
            m.targets[index] = (step % 4) as u32;
        }
        score = score + c.on_insert(&m, index, descriptor);
        assert_eq!(score, oracle(&m).0);
        assert_eq!(c.evaluate(&m), score);
        assert_eq!(c.match_count(&m), oracle(&m).1);
    }
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}
