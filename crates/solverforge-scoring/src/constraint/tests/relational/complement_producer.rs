use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::{count, CountAccumulator};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, ComplementNode, ComplementView, GroupNode, GroupView, Operator},
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
fn default(_: &Model, _: &Leaf<'_, u32>) -> Payload {
    Payload(-10)
}
fn weight<O: Operator<Model>>(
    _: &Model,
    row: &ComplementView<
        '_,
        Leaf<'_, u32>,
        GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
        Payload,
    >,
) -> SoftScore {
    match row {
        ComplementView::Real(pair) => pair.right.with_result(|r| SoftScore::of(*r as i64 * 5)),
        ComplementView::Default(projected) => SoftScore::of(projected.value.0),
    }
}
#[test]
fn complement_replaces_defaults_and_real_group_rows_without_clone_or_duplicate_scores() {
    let group = GroupNode::new(
        CollectionNode::new(
            source(
                entities as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(0),
            ),
            0,
        ),
        key,
        count(),
    );
    let targets = CollectionNode::new(
        source(targets as fn(&Model) -> &[u32], ChangeSource::Descriptor(1)),
        1,
    );
    let tree = ComplementNode::new(targets, group, equal_bi(target_key, group_key), default);
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "complement"),
        ImpactType::Reward,
        tree,
        weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1 }],
        targets: vec![1, 1, 2],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(0));
    assert_eq!(c.match_count(&m), 3);
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 0, 0);
    assert_eq!(score, SoftScore::of(-30));
    assert_eq!(c.on_retract(&m, 0, 0), SoftScore::of(0));
    m.entities[0].key = 2;
    assert_eq!(c.evaluate(&m), SoftScore::of(-15));
    score = score + c.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(-15));
    let matches = c.get_matches(&m);
    assert_eq!(matches.len(), 3);
    let mut lineage_sizes = matches
        .iter()
        .map(|m| m.justification.entities.len())
        .collect::<Vec<_>>();
    lineage_sizes.sort();
    assert_eq!(lineage_sizes, vec![1, 1, 2]);
    score = score + c.on_retract(&m, 0, 1);
    m.targets[0] = 2;
    score = score + c.on_insert(&m, 0, 1);
    assert_eq!(score, SoftScore::of(0));
    assert_eq!(score, c.evaluate(&m));
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}

fn output_key<O: Operator<Model>>(
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
fn complemented_rows_remain_joinable_through_repeated_target_descriptor_churn() {
    use crate::stream::relational::operator::{JoinNode, Pair};
    fn joined_weight<O: Operator<Model>>(
        m: &Model,
        row: &Pair<
            ComplementView<
                '_,
                Leaf<'_, u32>,
                GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
                Payload,
            >,
            Leaf<'_, u32>,
        >,
    ) -> SoftScore {
        weight(m, &row.left)
    }
    let group = GroupNode::new(
        CollectionNode::new(
            source(
                entities as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(0),
            ),
            0,
        ),
        key,
        count(),
    );
    let target = |binding| {
        CollectionNode::new(
            source(targets as fn(&Model) -> &[u32], ChangeSource::Descriptor(1)),
            binding,
        )
    };
    let complemented =
        ComplementNode::new(target(1), group, equal_bi(target_key, group_key), default);
    let tree = JoinNode::new(complemented, target(2), equal_bi(output_key, target_key));
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "joined complement"),
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
        let mut total = 0;
        let mut matches = 0;
        for a in &m.targets {
            let count = m.entities.iter().filter(|e| e.key == *a).count();
            let score = if count == 0 { -10 } else { count as i64 * 5 };
            for b in &m.targets {
                if a == b {
                    total += score;
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
        assert_eq!(
            c.get_matches(&m)
                .iter()
                .fold(SoftScore::of(0), |s, m| s + m.score),
            score
        );
    }
}
