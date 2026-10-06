/* Fluent join chains past the named arities (acceptance #5, fluent surface).

`Bi`/`Tri`/`Quad`/`Penta` are ergonomic adapters. `Penta::join` continues into
the arity-free `Chain`, whose `.join(...)` nests further `JoinNode`s with no
fixed ceiling. This drives a six-source fluent chain and checks scores against
a hand-computed oracle with exact incremental deltas.

Left keys are named `fn` items with explicit lifetimes so they are naturally
higher-ranked over the borrowed nested pair rows; closures over nested pair
rows would otherwise fix the lifetime and fail the `ExecutablePlan` bound.
*/

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::{equal_bi, equal_on};
use crate::stream::relational::operator::Pair;
use crate::stream::relational::Leaf;
use crate::stream::ConstraintFactory;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Value {
    key: u32,
}

struct Model {
    values: Vec<Value>,
}

fn values(m: &Model) -> &[Value] {
    m.values.as_slice()
}

// Depth 1 and its peers: every binding reads the same key field, and each
// left key is a named fn so it is higher-ranked over the borrowed row.
fn entity_key(v: &Value) -> u32 {
    v.key
}
fn leaf_key(row: &Leaf<'_, Value>) -> u32 {
    row.entity.key
}
fn pair_key(row: &Pair<Leaf<'_, Value>, Leaf<'_, Value>>) -> u32 {
    row.left.entity.key
}
fn tri_key(row: &Pair<Pair<Leaf<'_, Value>, Leaf<'_, Value>>, Leaf<'_, Value>>) -> u32 {
    row.left.left.entity.key
}
fn quad_key(
    row: &Pair<Pair<Pair<Leaf<'_, Value>, Leaf<'_, Value>>, Leaf<'_, Value>>, Leaf<'_, Value>>,
) -> u32 {
    row.left.left.left.entity.key
}
fn penta_key(
    row: &Pair<
        Pair<Pair<Pair<Leaf<'_, Value>, Leaf<'_, Value>>, Leaf<'_, Value>>, Leaf<'_, Value>>,
        Leaf<'_, Value>,
    >,
) -> u32 {
    row.left.left.left.left.entity.key
}

// Six-leaf nested pair row: five Pair levels.
type Row6<'a> = Pair<
    Pair<
        Pair<Pair<Pair<Leaf<'a, Value>, Leaf<'a, Value>>, Leaf<'a, Value>>, Leaf<'a, Value>>,
        Leaf<'a, Value>,
    >,
    Leaf<'a, Value>,
>;

fn six_weight<'a, 'b, 'c>(_: &'b Model, _: &'c Row6<'a>) -> SoftScore {
    SoftScore::of(1)
}

fn six_source_chain() -> impl IncrementalConstraint<Model, SoftScore> {
    ConstraintFactory::<Model, SoftScore>::new()
        .for_each(source(
            values as fn(&Model) -> &[Value],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_bi(entity_key, entity_key),
        ))
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(pair_key, leaf_key),
        ))
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(tri_key, leaf_key),
        ))
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(quad_key, leaf_key),
        ))
        // Depth 6: Penta::join enters the arity-free Chain.
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(penta_key, leaf_key),
        ))
        .penalize(six_weight)
        .named("six source chain")
}

fn oracle(m: &Model) -> SoftScore {
    // All six bindings share one descriptor; a match needs six equal keys.
    let mut matches = 0i64;
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
                                matches += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    SoftScore::of(-matches)
}

#[test]
fn six_source_fluent_chain_scores_and_retracts_exactly() {
    let mut m = Model {
        values: vec![Value { key: 1 }, Value { key: 1 }],
    };
    let c = six_source_chain();
    assert_eq!(c.evaluate(&m), oracle(&m));
    assert_eq!(c.match_count(&m) as i64, -oracle(&m).score());

    let mut c = six_source_chain();
    let mut score = c.initialize(&m);
    for step in 0..40 {
        let i = step % 2;
        score = score + c.on_retract(&m, i, 0);
        m.values[i].key = (step % 3) as u32;
        score = score + c.on_insert(&m, i, 0);
        assert_eq!(score, oracle(&m));
        assert_eq!(c.evaluate(&m), score);
    }
    c.reset();
    assert_eq!(c.evaluate(&m), score);
    assert_eq!(c.initialize(&m), score);
}

// Left key for a seventh join: reads the first binding off the six-leaf row.
fn row6_key<'a, 'b>(row: &'b Row6<'a>) -> u32 {
    row.left.left.left.left.left.entity.key
}

fn seven_source_chain() -> impl IncrementalConstraint<Model, SoftScore> {
    ConstraintFactory::<Model, SoftScore>::new()
        .for_each(source(
            values as fn(&Model) -> &[Value],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_bi(entity_key, entity_key),
        ))
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(pair_key, leaf_key),
        ))
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(tri_key, leaf_key),
        ))
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(quad_key, leaf_key),
        ))
        // depth 6
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(penta_key, leaf_key),
        ))
        // depth 7: a second Chain::join, no per-arity type involved.
        .join((
            source(
                values as fn(&Model) -> &[Value],
                ChangeSource::Descriptor(0),
            ),
            equal_on(row6_key, leaf_key),
        ))
        .filter(|_s: &Model, row: &Pair<Row7Leaf<'_>, Leaf<'_, Value>>| {
            // Filter at depth: only rows whose newest binding key is nonzero.
            row.right.entity.key > 0
        })
        .reward(seven_weight)
        .named("seven source chain")
}

type Row7<'a> = Pair<Row6<'a>, Leaf<'a, Value>>;
// Aliases used only to spell the filter view without deep nesting.
type Row7Leaf<'a> = Row6<'a>;

fn seven_weight<'a, 'b, 'c>(_: &'b Model, _: &'c Row7<'a>) -> SoftScore {
    SoftScore::of(1)
}

#[test]
fn seven_source_fluent_chain_with_filter_matches_oracle() {
    let m = Model {
        values: vec![Value { key: 1 }, Value { key: 1 }],
    };
    let c = seven_source_chain();
    // Seven bindings, one descriptor. A match needs all seven keys equal; the
    // depth-7 filter additionally requires the newest binding key > 0.
    // Two values with key 1: 2^7 = 128 combinations, all with key 1 > 0.
    assert_eq!(c.evaluate(&m), SoftScore::of(128));
    assert_eq!(c.match_count(&m), 128);
    // Dropping the key to 0 removes those rows entirely (filter rejects them).
    let m0 = Model {
        values: vec![Value { key: 1 }, Value { key: 0 }],
    };
    // matches: a1 x a1 x a1 x a1 x a1 x a0? No: all seven must share a key.
    // key=1 group size 1 -> 1; key=0 group size 1 -> rejected by the filter.
    assert_eq!(seven_source_chain().evaluate(&m0), SoftScore::of(1));
}
