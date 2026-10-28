// Weight multipliers across shared terminals, duplicate names, and resets.

use super::support::*;
use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collector::sum;

fn shift_source() -> impl crate::stream::collection_extract::CollectionExtract<Plan, Item = Shift> {
    source(shifts as fn(&Plan) -> &[Shift], SHIFTS)
}

fn shared_workload() -> impl ConstraintSet<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(shift_source())
        .group_by(
            |shift: &Shift| shift.employee.unwrap_or(usize::MAX),
            sum(|shift: &Shift| shift.hours),
        )
        .penalize(|_employee: &usize, hours: &i64| HardSoftScore::of_soft(*hours))
        .named("Workload")
        .reward(|employee: &usize, _hours: &i64| HardSoftScore::of_soft(*employee as i64 % 7))
        .named("Coverage")
}

fn unassigned() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(shift_source())
        .filter(|shift: &Shift| shift.employee.is_none())
        .penalize(HardSoftScore::of_hard(1))
        .named("Unassigned")
}

#[test]
fn shared_grouped_terminals_scale_independently() {
    for entries in [
        &[("Workload", 0), ("Coverage", 5)][..],
        &[("Workload", 3), ("Coverage", 0)][..],
        &[("Workload", 1)][..],
    ] {
        assert_weighted(shared_workload, entries);
    }
}

#[test]
fn shared_grouped_accumulator_state_ignores_weights() {
    let build = || {
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(shift_source())
            .group_by(
                |shift: &Shift| shift.employee.unwrap_or(usize::MAX),
                sum(|shift: &Shift| shift.hours),
            )
            .penalize(|_employee: &usize, hours: &i64| HardSoftScore::of_soft(*hours))
            .named("Workload")
            .reward(|employee: &usize, _hours: &i64| HardSoftScore::of_soft(*employee as i64 % 7))
            .named("Coverage")
    };
    let mut base = ScoreDirector::new(plan(ConstraintWeights::new()), build());
    let mut weights = ConstraintWeights::new();
    weights.set("Workload", 0).set("Coverage", 5);
    let mut weighted = ScoreDirector::new(plan(weights), build());
    base.calculate_score();
    weighted.calculate_score();
    for (descriptor, entity, mutate) in mutations() {
        base.do_change(descriptor, entity, mutate);
        weighted.do_change(descriptor, entity, mutate);
        assert_eq!(
            weighted.constraints().update_count(),
            base.constraints().update_count()
        );
        assert_eq!(
            weighted.constraints().changed_key_count(),
            base.constraints().changed_key_count()
        );
    }
}

#[test]
fn duplicate_names_scale_every_terminal() {
    let build = || {
        (
            unassigned(),
            ConstraintFactory::<Plan, HardSoftScore>::new()
                .for_each(shift_source())
                .filter(|shift: &Shift| shift.hours > 4)
                .penalize(HardSoftScore::of_hard(2))
                .named("Unassigned"),
        )
    };
    for multiplier in MULTIPLIERS {
        assert_weighted(build, &[("Unassigned", multiplier)]);
    }
}

#[test]
fn ordered_chains_bind_weights_by_structural_slot() {
    use crate::api::constraint_set::{ConstraintSetSource, OrderedConstraintSetChain};

    // Authored order puts the shared pair before the uni constraint; weight
    // slots follow the structure (left child first), so each constraint must
    // still receive its own multiplier.
    let build = || {
        OrderedConstraintSetChain::new(
            unassigned(),
            shared_workload(),
            vec![
                ConstraintSetSource::Right {
                    constraint_count: 2,
                    metadata_entry_count: 2,
                },
                ConstraintSetSource::Left,
            ],
        )
    };
    assert_weighted(
        build,
        &[("Unassigned", 0), ("Workload", 3), ("Coverage", 5)],
    );
    assert_weighted(build, &[("Unassigned", 2), ("Coverage", 0)]);
}

#[test]
fn disabled_hard_constraint_is_feasible_and_still_hard() {
    let mut weights = ConstraintWeights::new();
    weights.set("Unassigned", 0);
    let mut director = ScoreDirector::new(plan(weights), (unassigned(),));

    let score = director.calculate_score();
    assert_eq!(score, HardSoftScore::ZERO);
    assert!(score.is_feasible());
    let metadata = director.constraint_metadata();
    assert_eq!(metadata.len(), 1);
    assert!(metadata[0].is_hard);
    let results = director.constraint_results();
    assert_eq!(results[0].weight_multiplier, 0);
    assert_eq!(results[0].match_count, 1);
    assert_eq!(results[0].score, HardSoftScore::ZERO);
}

#[test]
fn reset_rebinds_changed_weights() {
    let mut director = ScoreDirector::new(plan(ConstraintWeights::new()), (unassigned(),));
    assert_eq!(director.calculate_score(), HardSoftScore::of_hard(-1));

    director.working_solution_mut().weights.set("Unassigned", 4);
    director.reset();
    assert_eq!(director.calculate_score(), HardSoftScore::of_hard(-4));
    assert_eq!(
        director.constraint_match_totals(),
        vec![("Unassigned".to_string(), 4, HardSoftScore::of_hard(-4), 1)]
    );

    director.working_solution_mut().weights.remove("Unassigned");
    director.reset();
    assert_eq!(director.calculate_score(), HardSoftScore::of_hard(-1));
}

#[test]
fn solution_without_weights_keeps_coded_scores() {
    #[derive(Clone)]
    struct Plain(Plan);

    impl solverforge_core::domain::PlanningSolution for Plain {
        type Score = HardSoftScore;
        fn score(&self) -> Option<HardSoftScore> {
            self.0.score
        }
        fn set_score(&mut self, score: Option<HardSoftScore>) {
            self.0.score = score;
        }
    }

    let mut weights = ConstraintWeights::new();
    weights.set("Unassigned", 9);
    let constraint = ConstraintFactory::<Plain, HardSoftScore>::new()
        .for_each(source(
            (|p: &Plain| p.0.shifts.as_slice()) as fn(&Plain) -> &[Shift],
            SHIFTS,
        ))
        .filter(|shift: &Shift| shift.employee.is_none())
        .penalize(HardSoftScore::of_hard(1))
        .named("Unassigned");
    let mut director = ScoreDirector::new(Plain(plan(weights)), (constraint,));
    assert_eq!(director.calculate_score(), HardSoftScore::of_hard(-1));
}
