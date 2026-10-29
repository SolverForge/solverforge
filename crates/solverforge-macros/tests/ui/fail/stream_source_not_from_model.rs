// A stream source must be a planning-model accessor; a `fn` item or a
// closure carries no descriptor and is rejected at compile time.

use solverforge::prelude::*;

struct Plan {
    shifts: Vec<usize>,
}

fn shifts(plan: &Plan) -> &[usize] {
    plan.shifts.as_slice()
}

fn main() {
    let _ = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(shifts as fn(&Plan) -> &[usize]);
    let _ = ConstraintFactory::<Plan, SoftScore>::new()
        .for_each(|plan: &Plan| plan.shifts.as_slice());
}
