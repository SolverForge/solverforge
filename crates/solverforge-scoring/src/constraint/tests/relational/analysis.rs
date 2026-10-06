/* Detailed-match analysis across the terminal lifecycle.

Explanations must agree with full evaluation before initialization, after
mutations, and after reset: every matched row contributes its contributor
identities and its signed score, and the signed sum equals the retained or
recomputed total. Contributor multiplicity (one descriptor repeated across a
row) must survive into the justification.
*/

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::{equal_bi, equal_on};
use crate::stream::relational::{operator::Pair, Leaf};
use crate::stream::ConstraintFactory;

use super::fixtures::{assignment_key, shift_key};
use super::fixtures::{
    rel_assignments, rel_employees, rel_shifts, sample, RelAssignment, RelEmployee, RelSchedule,
    RelShift,
};
use super::row_keys::{employee_key, row_pair_employee_code};

fn night_staffed() -> impl IncrementalConstraint<RelSchedule, SoftScore> {
    ConstraintFactory::<RelSchedule, SoftScore>::new()
        .for_each(source(
            rel_assignments as fn(&RelSchedule) -> &[RelAssignment],
            ChangeSource::Descriptor(1),
        ))
        .join((
            source(
                rel_shifts as fn(&RelSchedule) -> &[RelShift],
                ChangeSource::Descriptor(0),
            ),
            equal_bi(
                assignment_key as fn(&RelAssignment) -> u32,
                shift_key as fn(&RelShift) -> u32,
            ),
        ))
        .join((
            source(
                rel_employees as fn(&RelSchedule) -> &[RelEmployee],
                ChangeSource::Descriptor(2),
            ),
            equal_on(
                row_pair_employee_code
                    as fn(&Pair<Leaf<'_, RelAssignment>, Leaf<'_, RelShift>>) -> String,
                employee_key as fn(&Leaf<'_, RelEmployee>) -> String,
            ),
        ))
        .filter(|_a: &RelAssignment, shift: &RelShift, _e: &RelEmployee| shift.night)
        .penalize(|_a: &RelAssignment, _s: &RelShift, _e: &RelEmployee| SoftScore::of(1))
        .named("night shift staffed")
}

fn signed_sum(
    c: &impl IncrementalConstraint<RelSchedule, SoftScore>,
    s: &RelSchedule,
) -> SoftScore {
    c.get_matches(s)
        .iter()
        .fold(SoftScore::of(0), |acc, m| acc + m.score)
}

#[test]
fn explanations_agree_before_initialization() {
    let c = night_staffed();
    let s = sample();
    let matches = c.get_matches(&s);
    assert_eq!(matches.len(), 2);
    assert_eq!(signed_sum(&c, &s), c.evaluate(&s));
}

#[test]
fn explanation_carries_every_binding_of_a_row() {
    let c = night_staffed();
    let s = sample();
    // Each row is (assignment, shift, employee): three contributor entities.
    for m in c.get_matches(&s) {
        assert_eq!(m.justification.entities.len(), 3);
    }
}

#[test]
fn explanations_agree_after_mutation_and_reset() {
    let mut c = night_staffed();
    let s = sample();
    assert_eq!(c.initialize(&s), SoftScore::of(-2));
    assert_eq!(signed_sum(&c, &s), SoftScore::of(-2));

    // Retract a0 (descriptor 1, index 0): one row retracts.
    assert_eq!(c.on_retract(&s, 0, 1), SoftScore::of(1));
    // Explanation sums trace the post-retraction retained operator state.
    assert_eq!(signed_sum(&c, &s), SoftScore::of(-1));
    assert_eq!(c.get_matches(&s).len(), 1);

    // Re-insert a0: rows restore.
    assert_eq!(c.on_insert(&s, 0, 1), SoftScore::of(-1));
    assert_eq!(signed_sum(&c, &s), SoftScore::of(-2));
    assert_eq!(c.get_matches(&s).len(), 2);

    // Reset drops retention; explanations fall back to full evaluation.
    c.reset();
    assert_eq!(signed_sum(&c, &s), c.evaluate(&s));
    assert_eq!(c.get_matches(&s).len(), 2);
}
