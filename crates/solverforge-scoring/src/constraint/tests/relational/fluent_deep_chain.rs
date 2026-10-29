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

fn src() -> impl crate::stream::collection_extract::CollectionExtract<Model, Item = Value> {
    source(
        values as fn(&Model) -> &[Value],
        ChangeSource::Descriptor(0),
    )
}

// A six-source chain whose cross `Bi` keeps `bi_keep` rows and whose `Penta`
// keeps `penta_keep` rows, both before the sixth join enters the `Chain`.
fn filtered_six_source_chain(
    bi_keep: fn(&Value, &Value) -> bool,
    penta_keep: fn(&Value, &Value, &Value, &Value, &Value) -> bool,
) -> impl IncrementalConstraint<Model, SoftScore> {
    ConstraintFactory::<Model, SoftScore>::new()
        .for_each(src())
        .join((src(), equal_bi(entity_key, entity_key)))
        .filter(move |a: &Value, b: &Value| bi_keep(a, b))
        .join((src(), equal_on(pair_key, leaf_key)))
        .join((src(), equal_on(tri_key, leaf_key)))
        .join((src(), equal_on(quad_key, leaf_key)))
        .filter(
            move |a: &Value, b: &Value, c: &Value, d: &Value, e: &Value| penta_keep(a, b, c, d, e),
        )
        .join((src(), equal_on(penta_key, leaf_key)))
        .penalize(six_weight)
        .named("filtered six source chain")
}

fn filtered_oracle(
    m: &Model,
    bi_keep: fn(&Value, &Value) -> bool,
    penta_keep: fn(&Value, &Value, &Value, &Value, &Value) -> bool,
) -> SoftScore {
    let v = &m.values;
    let mut matches = 0i64;
    for a in v {
        for b in v.iter().filter(|b| b.key == a.key && bi_keep(a, b)) {
            for c in v.iter().filter(|c| c.key == a.key) {
                for d in v.iter().filter(|d| d.key == a.key) {
                    for e in v.iter().filter(|e| e.key == a.key) {
                        if !penta_keep(a, b, c, d, e) {
                            continue;
                        }
                        matches += v.iter().filter(|f| f.key == a.key).count() as i64;
                    }
                }
            }
        }
    }
    SoftScore::of(-matches)
}

fn keep_all_bi(_: &Value, _: &Value) -> bool {
    true
}
fn keep_all_penta(_: &Value, _: &Value, _: &Value, _: &Value, _: &Value) -> bool {
    true
}
fn reject_all_bi(_: &Value, _: &Value) -> bool {
    false
}
fn reject_all_penta(_: &Value, _: &Value, _: &Value, _: &Value, _: &Value) -> bool {
    false
}

#[test]
fn filters_before_the_sixth_join_reject_every_row() {
    for (bi_keep, penta_keep) in [
        (
            keep_all_bi as fn(&Value, &Value) -> bool,
            reject_all_penta as fn(&Value, &Value, &Value, &Value, &Value) -> bool,
        ),
        (reject_all_bi, keep_all_penta),
    ] {
        let mut m = Model {
            values: vec![Value { key: 1 }, Value { key: 1 }],
        };
        let c = filtered_six_source_chain(bi_keep, penta_keep);
        assert_eq!(c.evaluate(&m), SoftScore::ZERO);
        assert_eq!(c.match_count(&m), 0);

        let mut c = filtered_six_source_chain(bi_keep, penta_keep);
        assert_eq!(c.initialize(&m), SoftScore::ZERO);
        assert_eq!(c.on_retract(&m, 0, 0), SoftScore::ZERO);
        m.values[0].key = 2;
        assert_eq!(c.on_insert(&m, 0, 0), SoftScore::ZERO);
        assert_eq!(c.on_retract(&m, 0, 0), SoftScore::ZERO);
        m.values[0].key = 1;
        assert_eq!(c.on_insert(&m, 0, 0), SoftScore::ZERO);
        assert_eq!(c.evaluate(&m), SoftScore::ZERO);
    }
}

// Rejects pairs on key 2.
fn bi_not_two(a: &Value, _: &Value) -> bool {
    a.key != 2
}
// Rejects rows whose fifth binding has key 0.
fn penta_last_not_zero(_: &Value, _: &Value, _: &Value, _: &Value, e: &Value) -> bool {
    e.key != 0
}

#[test]
fn selective_bi_and_penta_filters_survive_the_sixth_join() {
    let mut m = Model {
        values: vec![Value { key: 0 }, Value { key: 1 }, Value { key: 2 }],
    };
    let expected = filtered_oracle(&m, bi_not_two, penta_last_not_zero);
    let c = filtered_six_source_chain(bi_not_two, penta_last_not_zero);
    assert_eq!(c.evaluate(&m), expected);
    assert_eq!(c.match_count(&m) as i64, -expected.score());

    let mut c = filtered_six_source_chain(bi_not_two, penta_last_not_zero);
    let mut score = c.initialize(&m);
    assert_eq!(score, expected);
    let mut saw_both_filters_reject = false;
    for step in 0..40 {
        let i = step % 3;
        score = score + c.on_retract(&m, i, 0);
        m.values[i].key = ((step * 7 + 1) % 3) as u32;
        score = score + c.on_insert(&m, i, 0);
        let expected = filtered_oracle(&m, bi_not_two, penta_last_not_zero);
        assert_eq!(score, expected, "step {step}");
        assert_eq!(c.evaluate(&m), score, "step {step}");
        let unfiltered = filtered_oracle(&m, keep_all_bi, keep_all_penta);
        let without_bi = filtered_oracle(&m, keep_all_bi, penta_last_not_zero);
        let without_penta = filtered_oracle(&m, bi_not_two, keep_all_penta);
        saw_both_filters_reject |=
            without_bi != expected && without_penta != expected && unfiltered != expected;
    }
    assert!(
        saw_both_filters_reject,
        "the walk must exercise both filters at once"
    );
}
