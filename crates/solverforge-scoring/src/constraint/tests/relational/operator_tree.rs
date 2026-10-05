use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::{equal_bi, filtering};
use crate::stream::relational::operator::{CollectionNode, JoinNode, Operator, Pair};
use crate::stream::relational::Leaf;

struct Model {
    values: Vec<i32>,
}
fn values(m: &Model) -> &[i32] {
    &m.values
}
fn leaf_key(v: &Leaf<'_, i32>) -> i32 {
    *v.entity
}

#[test]
fn arbitrary_tree_routes_one_descriptor_to_both_joined_branches_once() {
    let left = CollectionNode::new(
        source(values as fn(&Model) -> &[i32], ChangeSource::Descriptor(0)),
        0,
    );
    let right = CollectionNode::new(
        source(values as fn(&Model) -> &[i32], ChangeSource::Descriptor(0)),
        1,
    );
    let mut tree = JoinNode::new(left, right, equal_bi(leaf_key, leaf_key));
    let mut m = Model {
        values: vec![1, 1, 2],
    };
    Operator::initialize(&mut tree, &m);
    assert_eq!(tree.handles().len(), 5);
    let deltas = tree.retract(&m, 0, 0);
    assert_eq!(deltas.len(), 3);
    assert_eq!(tree.handles().len(), 2);
    assert!(tree.retract(&m, 0, 0).is_empty());
    m.values[0] = 2;
    let deltas = tree.insert(&m, 0, 0);
    assert_eq!(deltas.len(), 3);
    assert_eq!(tree.handles().len(), 5);
    assert!(tree.insert(&m, 0, 0).is_empty());
    for h in tree.handles() {
        let row = tree.resolve(&m, h).unwrap();
        assert_eq!(row.left.entity, row.right.entity);
    }
}

#[test]
fn joined_inputs_are_valid_on_both_sides_with_bag_multiplicity() {
    let source_node = |binding| {
        CollectionNode::new(
            source(values as fn(&Model) -> &[i32], ChangeSource::Descriptor(0)),
            binding,
        )
    };
    let left = JoinNode::new(source_node(0), source_node(1), equal_bi(leaf_key, leaf_key));
    let right = JoinNode::new(source_node(2), source_node(3), equal_bi(leaf_key, leaf_key));
    type Two<'a> = Pair<Leaf<'a, i32>, Leaf<'a, i32>>;
    let plan = equal_bi(|r: &Two<'_>| *r.left.entity, |r: &Two<'_>| *r.right.entity).and(
        filtering(|l: &Two<'_>, r: &Two<'_>| l.left.index != r.right.index),
    );
    let mut tree = JoinNode::new(left, right, plan);
    let m = Model { values: vec![7, 7] };
    Operator::initialize(&mut tree, &m);
    assert_eq!(Operator::handles(&tree).len(), 8);
    assert_eq!(Operator::retract(&mut tree, &m, 0, 0).len(), 8);
    assert!(Operator::handles(&tree).is_empty());
    assert_eq!(Operator::insert(&mut tree, &m, 0, 0).len(), 8);
}

#[test]
fn six_binding_tree_keeps_all_binding_provenance_and_targeted_deltas() {
    type One<'a> = Leaf<'a, i32>;
    type Two<'a> = Pair<One<'a>, One<'a>>;
    type Three<'a> = Pair<Two<'a>, One<'a>>;
    type Four<'a> = Pair<Three<'a>, One<'a>>;
    type Five<'a> = Pair<Four<'a>, One<'a>>;
    let leaf = |binding| {
        CollectionNode::new(
            source(values as fn(&Model) -> &[i32], ChangeSource::Descriptor(0)),
            binding,
        )
    };
    let two = JoinNode::new(leaf(0), leaf(1), equal_bi(leaf_key, leaf_key));
    let three = JoinNode::new(
        two,
        leaf(2),
        equal_bi(|r: &Two<'_>| *r.left.entity, leaf_key),
    );
    let four = JoinNode::new(
        three,
        leaf(3),
        equal_bi(|r: &Three<'_>| *r.right.entity, leaf_key),
    );
    let five = JoinNode::new(
        four,
        leaf(4),
        equal_bi(|r: &Four<'_>| *r.right.entity, leaf_key),
    );
    let mut six = JoinNode::new(
        five,
        leaf(5),
        equal_bi(|r: &Five<'_>| *r.right.entity, leaf_key),
    );
    let m = Model { values: vec![1, 1] };
    Operator::initialize(&mut six, &m);
    assert_eq!(Operator::handles(&six).len(), 64);
    let mut full_count = 0;
    Operator::visit_all(&six, &m, &mut |_| full_count += 1);
    assert_eq!(full_count, 64);
    for handle in Operator::handles(&six) {
        let mut provenance = Vec::new();
        Operator::visit_provenance(&six, handle, &mut |b, d, i| provenance.push((b, d, i)));
        assert_eq!(
            provenance.iter().map(|p| p.0).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4, 5]
        );
        assert!(provenance.iter().all(|p| p.1 == 0));
    }
    assert_eq!(Operator::retract(&mut six, &m, 0, 0).len(), 63);
    assert_eq!(Operator::handles(&six).len(), 1);
    assert_eq!(Operator::insert(&mut six, &m, 0, 0).len(), 63);
}

#[test]
fn borrowed_leaf_views_do_not_require_clone_or_copy_entities() {
    struct Entity(u32);
    let entity = Entity(7);
    let row = Leaf::new(&entity, 3);
    let another = row;
    assert_eq!(row.entity.0, another.entity.0);
    assert_eq!(row.index, another.index);
}
