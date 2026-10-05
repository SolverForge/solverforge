use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, FilterNode, JoinNode, Operator},
    Leaf,
};
struct Model {
    left: Vec<i32>,
    right: Vec<i32>,
}
fn left(m: &Model) -> &[i32] {
    &m.left
}
fn right(m: &Model) -> &[i32] {
    &m.right
}
fn key(v: &Leaf<'_, i32>) -> i32 {
    v.entity.abs()
}
#[test]
fn filters_on_both_inputs_preserve_membership_and_semantic_indexes() {
    let l = CollectionNode::new(
        source(left as fn(&Model) -> &[i32], ChangeSource::Descriptor(0)),
        0,
    );
    let r = CollectionNode::new(
        source(right as fn(&Model) -> &[i32], ChangeSource::Descriptor(1)),
        1,
    );
    let l = FilterNode::new(l, |_: &Model, v: &Leaf<'_, i32>| *v.entity > 0);
    let r = FilterNode::new(r, |_: &Model, v: &Leaf<'_, i32>| {
        *v.entity > 0 && v.index != 1
    });
    let mut tree = JoinNode::new(l, r, equal_bi(key, key));
    let mut m = Model {
        left: vec![-1, 1, 2],
        right: vec![1, 1, 2],
    };
    let check = |tree: &_, m: &Model, expected| {
        let mut count = 0;
        Operator::visit_all(tree, m, &mut |_| count += 1);
        assert_eq!(count, expected);
        assert_eq!(Operator::handles(tree).len(), expected);
    };
    Operator::initialize(&mut tree, &m);
    check(&tree, &m, 2);
    assert!(tree.retract(&m, 0, 0).is_empty());
    m.left[0] = 1;
    assert_eq!(tree.insert(&m, 0, 0).len(), 1);
    check(&tree, &m, 3);
    assert_eq!(tree.retract(&m, 0, 1).len(), 1);
    m.left[1] = -1;
    assert!(tree.insert(&m, 0, 1).is_empty());
    check(&tree, &m, 2);
    assert!(tree.retract(&m, 1, 1).is_empty());
    m.right[1] = 3;
    assert!(tree.insert(&m, 1, 1).is_empty());
    check(&tree, &m, 2);
}
