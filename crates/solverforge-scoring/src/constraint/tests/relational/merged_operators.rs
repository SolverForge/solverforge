use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, JoinNode, MergeNode, Operator},
    Leaf,
};
struct Model(Vec<u32>);
fn values(m: &Model) -> &[u32] {
    &m.0
}
fn key(v: &Leaf<'_, u32>) -> u32 {
    *v.entity
}
#[test]
fn merged_branch_ids_preserve_bags_and_invalidate_both_bindings() {
    let mut rm = Vec::new();
    let mut ins = Vec::new();
    let leaf = |binding| {
        CollectionNode::new(
            source(values as fn(&Model) -> &[u32], ChangeSource::Descriptor(0)),
            binding,
        )
    };
    let merge = MergeNode::new(leaf(0), leaf(1));
    let mut tree = JoinNode::new(merge, leaf(2), equal_bi(key, key));
    let mut m = Model(vec![7, 7]);
    tree.initialize(&m);
    assert_eq!(tree.handles().len(), 8);
    let mut full = 0;
    tree.visit_all(&m, &mut |_| full += 1);
    assert_eq!(full, 8);
    {
        tree.retract(&m, 0, 0, &mut rm, &mut ins);
        assert_eq!(rm.len() + ins.len(), 6);
    }
    {
        tree.retract(&m, 0, 0, &mut rm, &mut ins);
        assert!(rm.is_empty() && ins.is_empty());
    }
    m.0[0] = 8;
    {
        tree.insert(&m, 0, 0, &mut rm, &mut ins);
        assert_eq!(rm.len() + ins.len(), 2);
    }
    {
        tree.insert(&m, 0, 0, &mut rm, &mut ins);
        assert!(rm.is_empty() && ins.is_empty());
    }
    assert_eq!(tree.handles().len(), 4);
    let mut full = 0;
    tree.visit_all(&m, &mut |_| full += 1);
    assert_eq!(full, 4);
    for h in tree.handles() {
        let mut provenance = Vec::new();
        tree.visit_provenance(h, &mut |b, d, i| provenance.push((b, d, i)));
        assert_eq!(provenance.len(), 2);
        assert!(provenance[0].0 == 0 || provenance[0].0 == 1);
        assert_eq!(provenance[1].0, 2);
    }
}
