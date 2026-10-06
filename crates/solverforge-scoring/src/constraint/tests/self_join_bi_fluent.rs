// Self-join behavior on the fluent surface the runtime actually uses.
//
// Same-collection `.join(equal(key))` builds a `SelfJoinNode` through
// `JoinTarget`, so these tests exercise the migrated path end to end rather
// than the retired bespoke engine.

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal;
use crate::stream::ConstraintFactory;
use solverforge_core::score::SoftScore;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct Queen {
    row: i64,
    col: i64,
}

#[derive(Clone)]
struct NQueens {
    queens: Vec<Queen>,
}

fn queens(s: &NQueens) -> &[Queen] {
    s.queens.as_slice()
}

// Penal 1 per same-row ordered pair (a.col < b.col), keyed by row.
fn conflict_constraint() -> impl IncrementalConstraint<NQueens, SoftScore> {
    ConstraintFactory::<NQueens, SoftScore>::new()
        .for_each(source(
            queens as fn(&NQueens) -> &[Queen],
            ChangeSource::Descriptor(0),
        ))
        .join(equal(|q: &Queen| q.row))
        .filter(|a: &Queen, b: &Queen| a.col < b.col)
        .penalize(SoftScore::of(1))
        .named("Row conflict")
}

fn solution(queens: Vec<Queen>) -> NQueens {
    NQueens { queens }
}

#[test]
fn no_conflicts_scores_zero() {
    let c = conflict_constraint();
    let s = solution(vec![
        Queen { row: 0, col: 0 },
        Queen { row: 1, col: 1 },
        Queen { row: 2, col: 2 },
    ]);
    assert_eq!(c.evaluate(&s), SoftScore::of(0));
    assert_eq!(c.match_count(&s), 0);
}

#[test]
fn one_conflict_penalizes_once() {
    let c = conflict_constraint();
    let s = solution(vec![
        Queen { row: 0, col: 0 },
        Queen { row: 0, col: 1 },
        Queen { row: 2, col: 2 },
    ]);
    assert_eq!(c.evaluate(&s), SoftScore::of(-1));
    assert_eq!(c.match_count(&s), 1);
}

#[test]
fn three_on_a_row_yield_three_pairs() {
    let c = conflict_constraint();
    let s = solution(vec![
        Queen { row: 0, col: 0 },
        Queen { row: 0, col: 1 },
        Queen { row: 0, col: 2 },
    ]);
    assert_eq!(c.evaluate(&s), SoftScore::of(-3));
    assert_eq!(c.match_count(&s), 3);
}

#[test]
fn reward_type_scores_positive() {
    let c = ConstraintFactory::<NQueens, SoftScore>::new()
        .for_each(source(
            queens as fn(&NQueens) -> &[Queen],
            ChangeSource::Descriptor(0),
        ))
        .join(equal(|q: &Queen| q.row))
        .filter(|a: &Queen, b: &Queen| a.col < b.col)
        .reward(SoftScore::of(2))
        .named("Adjacent queens");
    let s = solution(vec![Queen { row: 0, col: 0 }, Queen { row: 0, col: 1 }]);
    assert_eq!(c.evaluate(&s), SoftScore::of(2));
}

#[test]
fn dynamic_weight_reads_both_entities() {
    let c = ConstraintFactory::<NQueens, SoftScore>::new()
        .for_each(source(
            queens as fn(&NQueens) -> &[Queen],
            ChangeSource::Descriptor(0),
        ))
        .join(equal(|q: &Queen| q.row))
        .filter(|a: &Queen, b: &Queen| a.col < b.col)
        .penalize(|a: &Queen, b: &Queen| SoftScore::of((b.col - a.col).abs()))
        .named("Column distance");
    let s = solution(vec![Queen { row: 0, col: 0 }, Queen { row: 0, col: 3 }]);
    assert_eq!(c.evaluate(&s), SoftScore::of(-3));
}

#[test]
fn incremental_insert_and_retract_track_full_evaluation() {
    let mut c = conflict_constraint();
    let s = solution(vec![
        Queen { row: 0, col: 0 },
        Queen { row: 0, col: 1 },
        Queen { row: 2, col: 2 },
    ]);

    c.initialize(&s);
    c.reset();

    // First queen: no pair yet.
    assert_eq!(c.on_insert(&s, 0, 0), SoftScore::of(0));
    // Second queen shares row 0: one new pair.
    assert_eq!(c.on_insert(&s, 1, 0), SoftScore::of(-1));
    // Third queen is alone on row 2: no new pair.
    assert_eq!(c.on_insert(&s, 2, 0), SoftScore::of(0));

    // Retracting the second queen removes its pair.
    assert_eq!(c.on_retract(&s, 1, 0), SoftScore::of(1));
}
