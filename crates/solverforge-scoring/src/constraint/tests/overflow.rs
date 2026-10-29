// Score deltas never wrap: the unchecked batch paths still panic at their
// public boundary when a level overflows.

use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::constraint::incremental::IncrementalUniConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone, Debug)]
struct Item {
    weight: i64,
}

struct Plan {
    items: Vec<Item>,
}

fn uni(
    impact: ImpactType,
) -> impl IncrementalConstraint<Plan, SoftScore> + ConstraintSet<Plan, SoftScore> {
    IncrementalUniConstraint::new(
        ConstraintRef::new("", "Weight"),
        impact,
        source(
            (|plan: &Plan| plan.items.as_slice()) as fn(&Plan) -> &[Item],
            ChangeSource::Descriptor(0),
        ),
        |_: &Plan, _: &Item| true,
        |item: &Item| SoftScore::of(item.weight),
        false,
    )
}

fn plan(weights: &[i64]) -> Plan {
    Plan {
        items: weights.iter().map(|&weight| Item { weight }).collect(),
    }
}

#[test]
#[should_panic(expected = "a score delta overflows i64")]
fn penalizing_a_minimum_weight_panics() {
    let plan = plan(&[i64::MIN]);
    let mut constraint = uni(ImpactType::Penalty);
    let _ = constraint.on_insert(&plan, 0, 0);
}

#[test]
#[should_panic(expected = "a score delta overflows i64")]
fn summing_constraint_deltas_past_i64_panics() {
    let plan = plan(&[i64::MAX]);
    let mut set = (uni(ImpactType::Reward), uni(ImpactType::Reward));
    let _ = set.on_insert_all(&plan, 0, 0);
}

#[test]
#[should_panic(expected = "a score delta overflows i64")]
fn weighted_delta_past_i64_panics() {
    let plan = plan(&[i64::MAX / 2 + 1]);
    let mut set = (uni(ImpactType::Reward),);
    let _ = set.on_insert_all_weighted(&plan, 0, 0, &[2]);
}

#[test]
fn in_range_deltas_are_unchanged() {
    let plan = plan(&[i64::MAX - 1, 5]);
    let mut set = (uni(ImpactType::Reward), uni(ImpactType::Penalty));
    assert_eq!(set.on_insert_all(&plan, 0, 0), SoftScore::of(0));
    assert_eq!(
        set.on_insert_all_weighted(&plan, 1, 0, &[3, 1]),
        SoftScore::of(10)
    );
}
