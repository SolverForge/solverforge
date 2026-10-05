#![allow(clippy::type_complexity)] // Test closures spell full row-view types deliberately.

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

#[test]
fn facade_group_result_can_join_and_publish_partial_group_replacement() {
    use solverforge::stream::collector::{count, CountAccumulator};
    use solverforge::stream::relational::operator::{GroupNode, GroupView, Operator};
    fn grouped_key<O: Operator<Model>>(
        row: &GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
    ) -> u32 {
        *row.key
    }
    fn group_weight<O: Operator<Model>>(
        _: &Model,
        row: &Pair<GroupView<'_, Model, O, u32, CountAccumulator, (), usize>, Leaf<'_, Entity>>,
    ) -> SoftScore {
        row.left.with_result(|count| SoftScore::of(*count as i64))
    }
    let leaf = |binding| {
        CollectionNode::new(
            source(
                entities as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let group = GroupNode::new(leaf(0), leaf_key, count());
    let tree = JoinNode::new(group, leaf(1), equal_bi(grouped_key, leaf_key));
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("public", "grouped"),
        ImpactType::Reward,
        tree,
        group_weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1, weight: 2 }, Entity { key: 1, weight: 3 }],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(4));
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 0, 0);
    assert_eq!(score, SoftScore::of(1));
    m.entities[0].key = 2;
    score = score + c.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(2));
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.get_matches(&m).len(), 2);
}

#[test]
fn facade_anti_join_updates_both_bindings_of_one_descriptor() {
    use solverforge::stream::joiner::filtering;
    use solverforge::stream::relational::operator::ExistenceNode;
    fn weight(_: &Model, row: &Leaf<'_, Entity>) -> SoftScore {
        SoftScore::of(row.entity.weight)
    }
    fn different(a: &Leaf<'_, Entity>, b: &Leaf<'_, Entity>) -> bool {
        a.index != b.index
    }
    let leaf = |binding| {
        CollectionNode::new(
            source(
                entities as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let condition = equal_bi(leaf_key, leaf_key).and(filtering(different));
    let tree = ExistenceNode::new(leaf(0), leaf(1), condition, false);
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("public", "anti"),
        ImpactType::Reward,
        tree,
        weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1, weight: 2 }, Entity { key: 1, weight: 3 }],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(0));
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 0, 0);
    assert_eq!(score, SoftScore::of(3));
    m.entities[0].key = 2;
    score = score + c.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(5));
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.match_count(&m), 2);
}

#[test]
fn facade_flatten_borrows_non_clone_children_from_owned_projection_evaluation() {
    use solverforge::stream::relational::operator::{FlattenNode, FlattenView};
    struct Parent(Vec<Payload>);
    fn parent(_: &Model, row: &Leaf<'_, Entity>) -> Vec<Parent> {
        vec![Parent(
            (0..2)
                .map(|_| Payload {
                    key: row.entity.key,
                    weight: row.entity.weight,
                })
                .collect(),
        )]
    }
    fn children<'a>(_: &'a Model, row: ProjectView<'a, Leaf<'a, Entity>, Parent>) -> &'a [Payload] {
        &row.value.0
    }
    fn child_key(row: &FlattenView<'_, ProjectView<'_, Leaf<'_, Entity>, Parent>, Payload>) -> u32 {
        row.value.key
    }
    fn weight(
        _: &Model,
        row: &Pair<
            FlattenView<'_, ProjectView<'_, Leaf<'_, Entity>, Parent>, Payload>,
            Leaf<'_, Entity>,
        >,
    ) -> SoftScore {
        SoftScore::of(row.left.value.weight)
    }
    let leaf = |binding| {
        CollectionNode::new(
            source(
                entities as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let flat = FlattenNode::new(ProjectNode::new(leaf(0), parent), children);
    let tree = JoinNode::new(flat, leaf(1), equal_bi(child_key, leaf_key));
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("public", "flatten"),
        ImpactType::Reward,
        tree,
        weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1, weight: 2 }, Entity { key: 1, weight: 3 }],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(20));
    assert_eq!(c.match_count(&m), 8);
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 0, 0);
    m.entities[0] = Entity { key: 2, weight: 9 };
    score = score + c.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(24));
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.match_count(&m), 4);
    assert!(c
        .get_matches(&m)
        .iter()
        .all(|x| x.justification.entities.len() == 2));
}

#[test]
fn facade_complement_owns_one_group_and_replaces_filtered_real_rows_with_defaults() {
    use solverforge::stream::collector::{count, CountAccumulator};
    use solverforge::stream::relational::operator::{
        ComplementNode, ComplementView, FilterNode, GroupNode, GroupView, Operator,
    };
    fn grouped_key<O: Operator<Model>>(
        row: &GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
    ) -> u32 {
        *row.key
    }
    fn default(_: &Model, row: &Leaf<'_, Entity>) -> Payload {
        Payload {
            key: row.entity.key,
            weight: -10,
        }
    }
    fn weight<O: Operator<Model>>(
        _: &Model,
        row: &ComplementView<
            '_,
            Leaf<'_, Entity>,
            GroupView<'_, Model, O, u32, CountAccumulator, (), usize>,
            Payload,
        >,
    ) -> SoftScore {
        match row {
            ComplementView::Real(pair) => pair
                .right
                .with_result(|count| SoftScore::of(*count as i64 * 5)),
            ComplementView::Default(projected) => SoftScore::of(projected.value.weight),
        }
    }
    fn accepted(_: &Model, row: &Leaf<'_, Entity>) -> bool {
        row.entity.weight > 0
    }
    let leaf = |binding| {
        CollectionNode::new(
            source(
                entities as fn(&Model) -> &[Entity],
                ChangeSource::Descriptor(0),
            ),
            binding,
        )
    };
    let grouped = GroupNode::new(FilterNode::new(leaf(0), accepted), leaf_key, count());
    let complemented =
        ComplementNode::new(leaf(1), grouped, equal_bi(leaf_key, grouped_key), default);
    let mut c = OperatorTerminal::new(
        ConstraintRef::new("public", "complement"),
        ImpactType::Reward,
        complemented,
        weight,
        false,
    );
    let mut m = Model {
        entities: vec![Entity { key: 1, weight: 0 }, Entity { key: 1, weight: 3 }],
    };
    assert_eq!(c.evaluate(&m), SoftScore::of(10));
    let mut score = c.initialize(&m);
    score = score + c.on_retract(&m, 1, 0);
    assert_eq!(score, SoftScore::of(-10));
    m.entities[1].weight = 0;
    score = score + c.on_insert(&m, 1, 0);
    assert_eq!(score, SoftScore::of(-20));
    assert_eq!(c.evaluate(&m), score);
    assert!(c
        .get_matches(&m)
        .iter()
        .all(|m| m.justification.entities.len() == 1));
}
