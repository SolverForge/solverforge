use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::ConstraintFactory;

#[derive(Clone)]
struct A {
    id: usize,
}

#[derive(Clone)]
struct B {
    id: usize,
}

#[derive(Clone)]
struct State {
    a: Vec<A>,
    b: Vec<B>,
}

fn as_a(state: &State) -> &[A] {
    state.a.as_slice()
}

fn as_b(state: &State) -> &[B] {
    state.b.as_slice()
}

#[test]
#[should_panic(expected = "cannot localize entity indexes")]
fn unknown_source_same_source_wrong_delta_on_wrong_descriptor_update() {
    let mut constraint = ConstraintFactory::<State, SoftScore>::new()
        .for_each(source(as_a as fn(&State) -> &[A], ChangeSource::Unknown))
        .filter(|_a: &A| true)
        .if_exists((
            ConstraintFactory::<State, SoftScore>::new()
                .for_each(source(as_b as fn(&State) -> &[B], ChangeSource::Unknown))
                .filter(|_b: &B| true),
            equal_bi(|a: &A| a.id, |b: &B| b.id),
        ))
        .penalize(SoftScore::of(1))
        .named("exists from unknown source");

    let mut state = State {
        a: vec![A { id: 2 }, A { id: 2 }],
        b: vec![B { id: 3 }, B { id: 2 }],
    };

    let initial = constraint.initialize(&state);
    assert_eq!(initial, SoftScore::of(-2));

    let mut total = initial;
    total = total + constraint.on_retract(&state, 1, 0);
    state.a[1].id = 3;

    total = total + constraint.on_insert(&state, 1, 0);

    assert_eq!(total, SoftScore::of(-1));
    assert_eq!(total, constraint.evaluate(&state));
}

#[test]
fn descriptor_exists_parent_side_update_matches_evaluate() {
    let mut constraint = ConstraintFactory::<State, SoftScore>::new()
        .for_each(source(
            as_a as fn(&State) -> &[A],
            ChangeSource::Descriptor(0),
        ))
        .if_exists((
            ConstraintFactory::<State, SoftScore>::new().for_each(source(
                as_b as fn(&State) -> &[B],
                ChangeSource::Descriptor(1),
            )),
            equal_bi(|a: &A| a.id, |b: &B| b.id),
        ))
        .penalize(SoftScore::of(1))
        .named("descriptor exists");

    let mut state = State {
        a: vec![A { id: 2 }, A { id: 2 }],
        b: vec![B { id: 3 }, B { id: 2 }],
    };

    let mut total = constraint.initialize(&state);
    assert_eq!(total, SoftScore::of(-2));

    total = total + constraint.on_retract(&state, 1, 1);
    state.b[1].id = 3;
    total = total + constraint.on_insert(&state, 1, 1);

    assert_eq!(total, SoftScore::of(0));
    assert_eq!(total, constraint.evaluate(&state));
}

#[test]
fn descriptor_not_exists_parent_side_update_matches_evaluate() {
    let mut constraint = ConstraintFactory::<State, SoftScore>::new()
        .for_each(source(
            as_a as fn(&State) -> &[A],
            ChangeSource::Descriptor(0),
        ))
        .if_not_exists((
            ConstraintFactory::<State, SoftScore>::new().for_each(source(
                as_b as fn(&State) -> &[B],
                ChangeSource::Descriptor(1),
            )),
            equal_bi(|a: &A| a.id, |b: &B| b.id),
        ))
        .penalize(SoftScore::of(1))
        .named("descriptor not exists");

    let mut state = State {
        a: vec![A { id: 2 }, A { id: 2 }],
        b: vec![B { id: 3 }, B { id: 2 }],
    };

    let mut total = constraint.initialize(&state);
    assert_eq!(total, SoftScore::of(0));

    total = total + constraint.on_retract(&state, 1, 1);
    state.b[1].id = 3;
    total = total + constraint.on_insert(&state, 1, 1);

    assert_eq!(total, SoftScore::of(-2));
    assert_eq!(total, constraint.evaluate(&state));
}
