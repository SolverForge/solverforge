use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::{
    plan::{CompileCondition, ExecutablePlan, IndexedPlan},
    *,
};
use crate::stream::relational::{
    operator::{CollectionNode, JoinNode, Operator},
    Leaf,
};

#[derive(Debug)]
struct Entity {
    key: i32,
    end: i32,
    enabled: bool,
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
fn key(v: &Leaf<'_, Entity>) -> i32 {
    v.entity.key
}
fn end(v: &Leaf<'_, Entity>) -> i32 {
    v.entity.end
}
fn verify<C>(condition: C, oracle: fn(&Entity, &Entity) -> bool)
where
    C: CompileCondition,
    C::Plan: IndexedPlan + 'static,
    for<'a> C::Plan: ExecutablePlan<Leaf<'a, Entity>, Leaf<'a, Entity>>,
{
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
    let mut tree = JoinNode::new(l, r, condition);
    let mut m = Model {
        left: Vec::new(),
        right: Vec::new(),
    };
    tree.initialize(&m);
    assert!(tree.handles().is_empty());
    m.left = vec![
        Entity {
            key: -1,
            end: 2,
            enabled: true,
        },
        Entity {
            key: 0,
            end: 0,
            enabled: false,
        },
        Entity {
            key: 0,
            end: 3,
            enabled: true,
        },
    ];
    m.right = vec![
        Entity {
            key: 0,
            end: 2,
            enabled: true,
        },
        Entity {
            key: 0,
            end: 2,
            enabled: true,
        },
        Entity {
            key: 2,
            end: -1,
            enabled: false,
        },
    ];
    tree.initialize(&m);
    type Node = CollectionNode<
        Model,
        crate::stream::collection_extract::SourceExtract<fn(&Model) -> &[Entity]>,
    >;
    let check = |tree: &JoinNode<Model, Node, Node, C::Plan>, m: &Model| {
        let mut expected = Vec::new();
        for (i, l) in m.left.iter().enumerate() {
            for (j, r) in m.right.iter().enumerate() {
                if oracle(l, r) {
                    expected.push((i, j));
                }
            }
        }
        let mut full = Vec::new();
        tree.visit_all(m, &mut |row| full.push((row.left.index, row.right.index)));
        let mut retained: Vec<_> = tree
            .handles()
            .into_iter()
            .map(|h| {
                let row = tree.resolve(m, h).unwrap();
                (row.left.index, row.right.index)
            })
            .collect();
        full.sort_unstable();
        retained.sort_unstable();
        assert_eq!(full, expected);
        assert_eq!(retained, expected);
    };
    check(&tree, &m);
    assert!(tree.retract(&m, 99, 0).is_empty());
    let mut seed = 19u64;
    for step in 0..120 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let descriptor = (step % 2) as usize;
        let index = (seed as usize) % 3;
        let removed = tree.retract(&m, descriptor, index);
        assert!(tree.retract(&m, descriptor, index).is_empty());
        for h in removed.removed {
            assert!(tree.resolve(&m, h).is_none());
        }
        let values = if descriptor == 0 {
            &mut m.left
        } else {
            &mut m.right
        };
        // Includes changes only to residual fields, unchanged indexed keys, and invalid intervals.
        if step % 3 == 0 {
            values[index].enabled = !values[index].enabled;
        } else if step % 3 == 1 {
            values[index].end = ((seed >> 12) % 7) as i32 - 3;
        } else {
            values[index].key = ((seed >> 8) % 7) as i32 - 3;
        }
        tree.insert(&m, descriptor, index);
        assert!(tree.insert(&m, descriptor, index).is_empty());
        check(&tree, &m);
    }
    tree.clear();
    assert!(tree.handles().is_empty());
    tree.initialize(&m);
    check(&tree, &m);
}
macro_rules! case {
    ($name:ident, $condition:expr, $oracle:expr) => {
        #[test]
        fn $name() {
            verify($condition, $oracle);
        }
    };
}
case!(equality_mutations, equal_bi(key, key), |l, r| l.key
    == r.key);
case!(less_mutations, less_than(key, key), |l, r| l.key < r.key);
case!(
    less_equal_mutations,
    less_than_or_equal(key, key),
    |l, r| l.key <= r.key
);
case!(greater_mutations, greater_than(key, key), |l, r| l.key
    > r.key);
case!(
    greater_equal_mutations,
    greater_than_or_equal(key, key),
    |l, r| l.key >= r.key
);
case!(
    overlap_mutations,
    overlapping(key, end, key, end),
    |l, r| l.key < r.end && r.key < l.end
);
case!(
    predicate_mutations,
    filtering(|l: &Leaf<'_, Entity>, r: &Leaf<'_, Entity>| l.entity.enabled && r.entity.enabled),
    |l, r| l.enabled && r.enabled
);
case!(
    equality_residual_mutations,
    equal_bi(key, key).and(filtering(|l: &Leaf<'_, Entity>, r: &Leaf<'_, Entity>| l
        .entity
        .enabled
        && r.entity.enabled)),
    |l, r| l.key == r.key && l.enabled && r.enabled
);
case!(
    reversed_residual_mutations,
    filtering(|l: &Leaf<'_, Entity>, r: &Leaf<'_, Entity>| l.entity.enabled && r.entity.enabled)
        .and(equal_bi(key, key)),
    |l, r| l.key == r.key && l.enabled && r.enabled
);
case!(
    composite_mutations,
    equal_bi(key, key).and(equal_bi(end, end)),
    |l, r| l.key == r.key && l.end == r.end
);
case!(
    interleaved_mutations,
    equal_bi(key, key)
        .and(filtering(|l: &Leaf<'_, Entity>, _: &Leaf<'_, Entity>| l
            .entity
            .enabled))
        .and(equal_bi(end, end)),
    |l, r| l.key == r.key && l.enabled && l.end == r.end
);
case!(
    equality_range_mutations,
    equal_bi(key, key).and(less_than(end, end)),
    |l, r| l.key == r.key && l.end < r.end
);
