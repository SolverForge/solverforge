use solverforge::prelude::*;
use solverforge::stream::relational::{operator::Pair, Leaf};

struct Model { a: Vec<u32>, b: Vec<u32>, c: Vec<u32> }
fn a(m: &Model) -> &[u32] { &m.a }
fn b(m: &Model) -> &[u32] { &m.b }
fn c(m: &Model) -> &[u32] { &m.c }
fn pair_key(row: &Pair<Leaf<'_, u32>, Leaf<'_, u32>>) -> u32 { *row.left.entity }
fn leaf_key(row: &Leaf<'_, u32>) -> u32 { *row.entity }

#[solverforge_constraints]
fn constraints() -> impl ConstraintSet<Model, SoftScore> {
    let factory = ConstraintFactory::<Model, SoftScore>::new();
    (
        factory.for_each(solverforge::stream::source(a as fn(&Model) -> &[u32], solverforge::stream::ChangeSource::Descriptor(0)))
            .join((solverforge::stream::source(b as fn(&Model) -> &[u32], solverforge::stream::ChangeSource::Static), joiner::equal_bi(|a: &u32| *a, |b: &u32| *b)))
            .join((solverforge::stream::source(c as fn(&Model) -> &[u32], solverforge::stream::ChangeSource::Static), joiner::equal_on(pair_key, leaf_key)))
            .group_by(|a: &u32, _: &u32, _: &u32| *a, count())
            .penalize(|_: &u32, n: &usize| SoftScore::of(*n as i64))
            .named("tri grouped count"),
    )
}
fn main() {
    let mut constraints = constraints();
    let model = Model { a: vec![1, 1], b: vec![1], c: vec![1] };
    assert_eq!(constraints.evaluate_all(&model), SoftScore::of(-2));
    assert_eq!(constraints.initialize_all(&model), SoftScore::of(-2));
}
