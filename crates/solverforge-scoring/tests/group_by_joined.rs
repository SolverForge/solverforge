use solverforge_core::score::SoftScore;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::stream::collector::count;
use solverforge_scoring::stream::joiner::{equal_bi, equal_on};
use solverforge_scoring::stream::relational::{operator::Pair, Leaf};
use solverforge_scoring::stream::{source, ChangeSource, ConstraintFactory};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Item {
    key: u32,
    value: i64,
}
struct Model {
    a: Vec<Item>,
    b: Vec<Item>,
    c: Vec<Item>,
    d: Vec<Item>,
    e: Vec<Item>,
}
fn a(m: &Model) -> &[Item] {
    &m.a
}
fn b(m: &Model) -> &[Item] {
    &m.b
}
fn c(m: &Model) -> &[Item] {
    &m.c
}
fn pair_key(row: &Pair<Leaf<'_, Item>, Leaf<'_, Item>>) -> u32 {
    row.left.entity.key
}
fn leaf_key(row: &Leaf<'_, Item>) -> u32 {
    row.entity.key
}

#[test]
fn self_join_tri_group_by_counts_combinations() {
    let constraint = solverforge_scoring::stream::TriConstraintStream::<
        Model,
        Item,
        u32,
        _,
        _,
        _,
        SoftScore,
    >::new_self_join(
        source(a as fn(&Model) -> &[Item], ChangeSource::Descriptor(0)),
        |_: &Model, i: &Item, _: usize| i.key,
    )
    .group_by(|i: &Item, _: &Item, _: &Item| i.key, count())
    .reward(|_: &u32, n: &usize| SoftScore::of(*n as i64))
    .named("self tri groups");
    let model = Model {
        a: vec![Item { key: 1, value: 1 }; 5],
        b: vec![],
        c: vec![],
        d: vec![],
        e: vec![],
    };
    assert_eq!(constraint.evaluate(&model), SoftScore::of(10));
}

#[test]
fn self_join_quad_group_by_counts_combinations() {
    let mut constraint = solverforge_scoring::stream::QuadConstraintStream::<
        Model,
        Item,
        u32,
        _,
        _,
        _,
        SoftScore,
    >::new_self_join(
        source(a as fn(&Model) -> &[Item], ChangeSource::Descriptor(0)),
        |_: &Model, i: &Item, _: usize| i.key,
    )
    .group_by(|i: &Item, _: &Item, _: &Item, _: &Item| i.key, count())
    .reward(|_: &u32, n: &usize| SoftScore::of(*n as i64))
    .named("self quad groups");
    let mut m = model();
    m.a = vec![Item { key: 1, value: 1 }; 5];
    let mut score = constraint.initialize(&m);
    assert_eq!(score, SoftScore::of(5));
    score = score + constraint.on_retract(&m, 0, 0);
    m.a[0].key = 2;
    score = score + constraint.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(1));
    assert_eq!(score, constraint.evaluate(&m));
}

#[test]
fn self_join_penta_group_by_counts_combinations() {
    let mut constraint = solverforge_scoring::stream::PentaConstraintStream::<
        Model,
        Item,
        u32,
        _,
        _,
        _,
        SoftScore,
    >::new_self_join(
        source(a as fn(&Model) -> &[Item], ChangeSource::Descriptor(0)),
        |_: &Model, i: &Item, _: usize| i.key,
    )
    .group_by(
        |i: &Item, _: &Item, _: &Item, _: &Item, _: &Item| i.key,
        count(),
    )
    .reward(|_: &u32, n: &usize| SoftScore::of(*n as i64))
    .named("self penta groups");
    let mut m = model();
    m.a = vec![Item { key: 1, value: 1 }; 5];
    let mut score = constraint.initialize(&m);
    assert_eq!(score, SoftScore::of(1));
    score = score + constraint.on_retract(&m, 0, 0);
    m.a[0].key = 2;
    score = score + constraint.on_insert(&m, 0, 0);
    assert_eq!(score, SoftScore::of(0));
    assert_eq!(score, constraint.evaluate(&m));
}

#[test]
fn tri_group_by_preserves_filter_and_incremental_counts() {
    let mut constraint = ConstraintFactory::<Model, SoftScore>::new()
        .for_each(source(
            a as fn(&Model) -> &[Item],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(b as fn(&Model) -> &[Item], ChangeSource::Descriptor(1)),
            equal_bi(|a: &Item| a.key, |b: &Item| b.key),
        ))
        .join((
            source(c as fn(&Model) -> &[Item], ChangeSource::Descriptor(2)),
            equal_on(pair_key, leaf_key),
        ))
        .filter(|a: &Item, _: &Item, _: &Item| a.value > 0)
        .group_by(|a: &Item, _: &Item, _: &Item| a.key, count())
        .penalize(|_: &u32, n: &usize| SoftScore::of(*n as i64))
        .named("tri groups");
    let mut model = Model {
        a: vec![Item { key: 1, value: 1 }, Item { key: 1, value: -1 }],
        b: vec![Item { key: 1, value: 2 }, Item { key: 1, value: 2 }],
        c: vec![Item { key: 1, value: 3 }],
        d: vec![],
        e: vec![],
    };
    assert_eq!(constraint.evaluate(&model), SoftScore::of(-2));
    assert_eq!(constraint.match_count(&model), 1);
    let mut score = constraint.initialize(&model);
    score = score + constraint.on_retract(&model, 0, 0);
    model.a[0].key = 2;
    score = score + constraint.on_insert(&model, 0, 0);
    assert_eq!(score, SoftScore::of(0));
    assert_eq!(score, constraint.evaluate(&model));
    score = score + constraint.on_retract(&model, 0, 0);
    model.a[0].key = 1;
    score = score + constraint.on_insert(&model, 0, 0);
    assert_eq!(score, SoftScore::of(-2));
    constraint.reset();
    assert_eq!(constraint.initialize(&model), score);
}

fn d(m: &Model) -> &[Item] {
    &m.d
}
fn e(m: &Model) -> &[Item] {
    &m.e
}
type TriRow<'a> = Pair<Pair<Leaf<'a, Item>, Leaf<'a, Item>>, Leaf<'a, Item>>;
type QuadRow<'a> = Pair<TriRow<'a>, Leaf<'a, Item>>;
fn tri_key(row: &TriRow<'_>) -> u32 {
    row.right.entity.key
}
fn quad_key(row: &QuadRow<'_>) -> u32 {
    row.right.entity.key
}

macro_rules! tri {
    () => {
        ConstraintFactory::<Model, SoftScore>::new()
            .for_each(source(
                a as fn(&Model) -> &[Item],
                ChangeSource::Descriptor(0),
            ))
            .join((
                source(b as fn(&Model) -> &[Item], ChangeSource::Descriptor(1)),
                equal_bi(|a: &Item| a.key, |b: &Item| b.key),
            ))
            .join((
                source(c as fn(&Model) -> &[Item], ChangeSource::Descriptor(2)),
                equal_on(pair_key, leaf_key),
            ))
    };
}
macro_rules! quad {
    () => {
        tri!().join((
            source(d as fn(&Model) -> &[Item], ChangeSource::Descriptor(3)),
            equal_on(tri_key, leaf_key),
        ))
    };
}
macro_rules! penta {
    () => {
        quad!().join((
            source(e as fn(&Model) -> &[Item], ChangeSource::Descriptor(4)),
            equal_on(quad_key, leaf_key),
        ))
    };
}

fn model() -> Model {
    Model {
        a: vec![Item { key: 1, value: 1 }],
        b: vec![Item { key: 1, value: 2 }; 2],
        c: vec![Item { key: 1, value: 3 }],
        d: vec![Item { key: 1, value: 4 }],
        e: vec![Item { key: 1, value: 5 }],
    }
}
fn verify_sources(
    mut constraint: impl IncrementalConstraint<Model, SoftScore>,
    sources: usize,
    expected: SoftScore,
) {
    let mut m = model();
    assert_eq!(constraint.evaluate(&m), expected);
    assert_eq!(constraint.match_count(&m), 1);
    let mut score = constraint.initialize(&m);
    assert_eq!(score, expected);
    for descriptor in 0..sources {
        score = score + constraint.on_retract(&m, 0, descriptor);
        assert_eq!(constraint.on_retract(&m, 0, descriptor), SoftScore::of(0));
        let rows = match descriptor {
            0 => &mut m.a,
            1 => &mut m.b,
            2 => &mut m.c,
            3 => &mut m.d,
            _ => &mut m.e,
        };
        rows[0].key = 2;
        score = score + constraint.on_insert(&m, 0, descriptor);
        assert_eq!(score, constraint.evaluate(&m));
        score = score + constraint.on_retract(&m, 0, descriptor);
        let rows = match descriptor {
            0 => &mut m.a,
            1 => &mut m.b,
            2 => &mut m.c,
            3 => &mut m.d,
            _ => &mut m.e,
        };
        rows[0].key = 1;
        score = score + constraint.on_insert(&m, 0, descriptor);
        assert_eq!(score, expected);
        assert_eq!(score, constraint.evaluate(&m));
    }
    constraint.reset();
    assert_eq!(constraint.initialize(&m), expected);
    assert_eq!(constraint.get_matches(&m).len(), 1);
}

#[test]
fn quad_group_by_retracts_each_source() {
    let constraint = quad!()
        .group_by(|_: &Item, _: &Item, _: &Item, d: &Item| d.key, count())
        .reward(|_: &u32, n: &usize| SoftScore::of(*n as i64))
        .named("quad groups");
    verify_sources(constraint, 4, SoftScore::of(2));
}

struct Payload(i64);
#[test]
fn penta_group_by_collects_move_only_payloads_and_retracts_each_source() {
    use solverforge_scoring::stream::collector::{collect_vec, CollectedVec};
    let constraint = penta!()
        .group_by(
            |_: &Item, _: &Item, _: &Item, _: &Item, e: &Item| e.key,
            collect_vec(|(a, b, c, d, e): (&Item, &Item, &Item, &Item, &Item)| {
                Payload(a.value + b.value + c.value + d.value + e.value)
            }),
        )
        .penalize(|_: &u32, result: &CollectedVec<Payload>| {
            SoftScore::of(result.iter().map(|p| p.0).sum())
        })
        .named("penta payload groups");
    verify_sources(constraint, 5, SoftScore::of(-30));
}

#[test]
fn joined_group_by_preserves_reject_all_filters_at_every_arity() {
    let tri = tri!()
        .filter(|_: &Item, _: &Item, _: &Item| false)
        .group_by(|a: &Item, _: &Item, _: &Item| a.key, count())
        .penalize(SoftScore::of(1))
        .named("empty tri");
    let quad = quad!()
        .filter(|_: &Item, _: &Item, _: &Item, _: &Item| false)
        .group_by(|a: &Item, _: &Item, _: &Item, _: &Item| a.key, count())
        .penalize(SoftScore::of(1))
        .named("empty quad");
    let penta = penta!()
        .filter(|_: &Item, _: &Item, _: &Item, _: &Item, _: &Item| false)
        .group_by(
            |a: &Item, _: &Item, _: &Item, _: &Item, _: &Item| a.key,
            count(),
        )
        .penalize(SoftScore::of(1))
        .named("empty penta");
    fn assert_empty(mut constraint: impl IncrementalConstraint<Model, SoftScore>) {
        let m = model();
        assert_eq!(constraint.evaluate(&m), SoftScore::of(0));
        assert_eq!(constraint.match_count(&m), 0);
        assert_eq!(constraint.initialize(&m), SoftScore::of(0));
    }
    assert_empty(tri);
    assert_empty(quad);
    assert_empty(penta);
}
