// The director's weight mode: chosen by type, switched only by rebuilding.

use std::any::TypeId;

use super::support::*;
use crate::api::constraint_set::IncrementalConstraint;
use crate::director::score_director::{requires_weighted_mode, ReplicaDescriptor, UnweightedMode};
use solverforge_core::domain::SolutionDescriptor;

fn unassigned() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(source(shifts as fn(&Plan) -> &[Shift], SHIFTS))
        .filter(|shift: &Shift| shift.employee.is_none())
        .penalize(HardSoftScore::of_hard(1))
        .named("Unassigned")
}

fn unassigned_weighted(multiplier: u32) -> Plan {
    let mut weights = ConstraintWeights::new();
    weights.set("Unassigned", multiplier);
    plan(weights)
}

#[test]
fn unweighted_mode_ignores_solution_weights() {
    let mut director = ScoreDirector::new(unassigned_weighted(9), (unassigned(),));

    assert_eq!(director.calculate_score(), HardSoftScore::of_hard(-1));
    assert_eq!(director.fresh_score(), Some(HardSoftScore::of_hard(-1)));
    assert_eq!(director.constraint_results()[0].weight_multiplier, 1);

    director.do_change(0, 0, |p| p.shifts[0].employee = None);
    assert_eq!(director.get_score(), HardSoftScore::of_hard(-2));
}

#[test]
fn requires_weighted_mode_only_for_a_multiplier_other_than_one() {
    assert!(!requires_weighted_mode(&plan(ConstraintWeights::new())));
    assert!(!requires_weighted_mode(&unassigned_weighted(1)));
    assert!(requires_weighted_mode(&unassigned_weighted(0)));
    assert!(requires_weighted_mode(&unassigned_weighted(3)));
}

#[test]
fn into_weight_mode_rescores_under_the_other_mode() {
    let mut unweighted = ScoreDirector::new(unassigned_weighted(4), (unassigned(),));
    assert_eq!(unweighted.calculate_score(), HardSoftScore::of_hard(-1));

    let mut weighted = unweighted.into_weight_mode::<WeightedMode>();
    assert!(!weighted.is_initialized());
    assert_eq!(weighted.calculate_score(), HardSoftScore::of_hard(-4));
    weighted.do_change(0, 0, |p| p.shifts[0].employee = None);
    assert_eq!(weighted.get_score(), HardSoftScore::of_hard(-8));
    assert_eq!(weighted.fresh_score(), Some(HardSoftScore::of_hard(-8)));

    let mut back = weighted.into_weight_mode::<UnweightedMode>();
    assert_eq!(back.calculate_score(), HardSoftScore::of_hard(-2));
    assert_eq!(back.fresh_score(), Some(HardSoftScore::of_hard(-2)));
}

#[test]
fn fork_replica_keeps_the_weighted_mode() {
    let descriptor = SolutionDescriptor::new("", TypeId::of::<()>());
    let build = (|_: &Plan| (unassigned(),)) as fn(&Plan) -> _;
    let solution = unassigned_weighted(5);
    let director = ScoreDirector::with_descriptor_weighted(
        solution.clone(),
        build(&solution),
        ReplicaDescriptor::new(&descriptor, build),
        |_, _| 0,
    );

    let mut replica = director
        .fork_replica()
        .expect("a replica descriptor forks replicas");
    assert_eq!(replica.get_score(), HardSoftScore::of_hard(-5));
    replica.do_change(0, 1, |p| p.shifts[1].employee = None);
    assert_eq!(replica.get_score(), HardSoftScore::of_hard(-10));
}
