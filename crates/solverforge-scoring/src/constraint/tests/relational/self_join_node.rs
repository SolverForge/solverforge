// SelfJoinNode: unique ordered index combinations within one collection.
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::relational::operator::{CollectionNode, Operator, SelfJoinNode};
use crate::stream::relational::Leaf;

#[derive(Clone, Debug)]
struct Item {
    key: u32,
}

#[derive(Clone)]
struct Model {
    items: Vec<Item>,
}

fn items(m: &Model) -> &[Item] {
    m.items.as_slice()
}

type Source = crate::stream::collection_extract::SourceExtract<fn(&Model) -> &[Item]>;
type Node = SelfJoinNode<
    Model,
    Item,
    CollectionNode<Model, Source>,
    u32,
    fn(&Model, &Item, usize) -> u32,
    2,
>;

fn node() -> Node {
    let input = CollectionNode::new(
        source(items as fn(&Model) -> &[Item], ChangeSource::Descriptor(0)),
        0,
    );
    let key = (|_s: &Model, i: &Item, _idx: usize| i.key) as fn(&Model, &Item, usize) -> u32;
    SelfJoinNode::new(input, key)
}

fn rows(n: &Node, m: &Model) -> usize {
    let mut c = 0;
    n.visit_all(m, &mut |_: [Leaf<'_, Item>; 2]| c += 1);
    c
}

#[test]
fn enumerates_unique_pairs_per_key() {
    let m = Model {
        items: vec![
            Item { key: 1 },
            Item { key: 1 },
            Item { key: 1 },
            Item { key: 2 },
        ],
    };
    let n = node();
    // key 1: C(3,2)=3 pairs; key 2: 0.
    assert_eq!(rows(&n, &m), 3);
}

#[test]
fn incremental_insert_and_retract_match_full_evaluation() {
    let mut m = Model {
        items: vec![Item { key: 1 }, Item { key: 1 }],
    };
    let mut n = node();
    n.initialize(&m);
    assert_eq!(n.handles().len(), 1);

    // Insert a third key-1 item: two new pairs appear.
    m.items.push(Item { key: 1 });
    let changes = n.insert(&m, 0, 2);
    assert_eq!(changes.inserted.len(), 2);
    assert_eq!(n.handles().len(), 3);
    assert_eq!(n.handles().len(), rows(&n, &m));

    // Retract it: both pairs disappear.
    m.items.pop();
    let changes = n.retract(&m, 0, 2);
    assert_eq!(changes.removed.len(), 2);
    assert_eq!(n.handles().len(), 1);
    assert_eq!(n.handles().len(), rows(&n, &m));
}

// Unique triples: the same node at arity 3 enumerates C(n, 3), not ordered
// Cartesian triples — the uniqueness policy has no arity ceiling.
type TriNode = SelfJoinNode<
    Model,
    Item,
    CollectionNode<Model, Source>,
    u32,
    fn(&Model, &Item, usize) -> u32,
    3,
>;

fn tri_node() -> TriNode {
    let input = CollectionNode::new(
        source(items as fn(&Model) -> &[Item], ChangeSource::Descriptor(0)),
        0,
    );
    let key = (|_s: &Model, i: &Item, _idx: usize| i.key) as fn(&Model, &Item, usize) -> u32;
    SelfJoinNode::new(input, key)
}

fn tri_rows(n: &TriNode, m: &Model) -> usize {
    let mut c = 0;
    n.visit_all(m, &mut |_: [Leaf<'_, Item>; 3]| c += 1);
    c
}

#[test]
fn unique_triples_per_key() {
    let m = Model {
        items: vec![
            Item { key: 1 },
            Item { key: 1 },
            Item { key: 1 },
            Item { key: 1 },
            Item { key: 2 },
        ],
    };
    let n = tri_node();
    // key 1: C(4,3)=4 triples; key 2: 0.
    assert_eq!(tri_rows(&n, &m), 4);
}

#[test]
fn triple_incremental_matches_full_evaluation() {
    let mut m = Model {
        items: vec![Item { key: 1 }, Item { key: 1 }, Item { key: 1 }],
    };
    let mut n = tri_node();
    n.initialize(&m);
    assert_eq!(n.handles().len(), 1); // C(3,3)=1

    // A fourth key-1 item adds C(3,2)=3 new triples.
    m.items.push(Item { key: 1 });
    let changes = n.insert(&m, 0, 3);
    assert_eq!(changes.inserted.len(), 3);
    assert_eq!(n.handles().len(), 4);
    assert_eq!(n.handles().len(), tri_rows(&n, &m));

    // Remove the second item: every triple containing it retracts (3 of them).
    m.items.remove(1);
    let changes = n.retract(&m, 0, 1);
    assert_eq!(changes.removed.len(), 3);
    assert_eq!(n.handles().len(), 1); // C(3,3)=1 remains
    assert_eq!(n.handles().len(), tri_rows(&n, &m));
}
