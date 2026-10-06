/* Arbitrary typed join depth (acceptance #5).

The internal row representation is a binary nested `Pair`: it has no fixed
arity ceiling, and depth beyond the ergonomic Uni/Bi/Tri/Quad/Penta adapters
is expressed as nested typed views, never erased values. This exercises a
six-binding join chain that reuses descriptor 0 in several bindings and
verifies exact incremental deltas against an oracle.
*/

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::relational::{
    operator::{CollectionNode, JoinNode, Pair},
    Leaf,
};
use solverforge_core::{score::SoftScore, ConstraintRef, ImpactType};

#[derive(Clone, Debug)]
struct Value {
    key: u32,
    weight: i64,
}

struct Model {
    values: Vec<Value>,
}

fn values(m: &Model) -> &[Value] {
    &m.values
}
fn key(v: &Leaf<'_, Value>) -> u32 {
    v.entity.key
}
fn leaf(
    binding: u32,
) -> CollectionNode<Model, crate::stream::collection_extract::SourceExtract<fn(&Model) -> &[Value]>>
{
    CollectionNode::new(
        source(
            values as fn(&Model) -> &[Value],
            ChangeSource::Descriptor(0),
        ),
        binding,
    )
}

// Six bindings, five nested joins, one reused descriptor.
type L<'a> = Leaf<'a, Value>;
type P1<'a> = Pair<L<'a>, L<'a>>;
type P2<'a> = Pair<P1<'a>, L<'a>>;
type P3<'a> = Pair<P2<'a>, L<'a>>;
type P4<'a> = Pair<P3<'a>, L<'a>>;
type P5<'a> = Pair<P4<'a>, L<'a>>;

fn depth6_weight(_: &Model, row: &P5<'_>) -> SoftScore {
    let sum = row.left.left.left.left.left.entity.weight
        + row.left.left.left.left.right.entity.weight
        + row.left.left.left.right.entity.weight
        + row.left.left.right.entity.weight
        + row.left.right.entity.weight
        + row.right.entity.weight;
    SoftScore::of(sum)
}

#[test]
fn six_binding_typed_join_chain_matches_oracle_and_retracts_exactly() {
    let n0 = leaf(0);
    let n1 = leaf(1);
    let n2 = leaf(2);
    let n3 = leaf(3);
    let n4 = leaf(4);
    let n5 = leaf(5);

    let j1 = JoinNode::new(n0, n1, equal_bi(key, key));
    let j2 = JoinNode::new(j1, n2, equal_bi(|v: &P1<'_>| v.left.entity.key, key));
    let j3 = JoinNode::new(j2, n3, equal_bi(|v: &P2<'_>| v.left.left.entity.key, key));
    let j4 = JoinNode::new(
        j3,
        n4,
        equal_bi(|v: &P3<'_>| v.left.left.left.entity.key, key),
    );
    let j5 = JoinNode::new(
        j4,
        n5,
        equal_bi(|v: &P4<'_>| v.left.left.left.left.entity.key, key),
    );

    let mut c = OperatorTerminal::new(
        ConstraintRef::new("test", "depth6"),
        ImpactType::Penalty,
        j5,
        depth6_weight,
        false,
    );
    let mut m = Model {
        values: vec![Value { key: 1, weight: 1 }, Value { key: 1, weight: 2 }],
    };
    let oracle = |m: &Model| {
        let mut sum = 0i64;
        let mut matches = 0i64;
        // All six bindings share descriptor 0; a match needs six equal keys.
        for a in &m.values {
            for b in &m.values {
                for d in &m.values {
                    for e in &m.values {
                        for f in &m.values {
                            for g in &m.values {
                                if a.key == b.key
                                    && b.key == d.key
                                    && d.key == e.key
                                    && e.key == f.key
                                    && f.key == g.key
                                {
                                    sum += a.weight
                                        + b.weight
                                        + d.weight
                                        + e.weight
                                        + f.weight
                                        + g.weight;
                                    matches += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        (SoftScore::of(-sum), matches)
    };
    assert_eq!(c.evaluate(&m), oracle(&m).0);
    assert_eq!(c.match_count(&m) as i64, oracle(&m).1);
    let mut score = c.initialize(&m);
    for step in 0..60 {
        let i = step % 2;
        score = score + c.on_retract(&m, i, 0);
        m.values[i].key = (step % 3) as u32;
        m.values[i].weight = step as i64 - 30;
        score = score + c.on_insert(&m, i, 0);
        assert_eq!(score, oracle(&m).0);
        assert_eq!(c.evaluate(&m), score);
        assert_eq!(c.match_count(&m) as i64, oracle(&m).1);
    }
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}
