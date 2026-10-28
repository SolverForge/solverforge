#[path = "constraint_weights_solution/domain/mod.rs"]
mod domain;

use domain::*;
use solverforge::__internal::PlanningSolution;
use solverforge::ConstraintWeights;

const _: () = assert!(<Plan as PlanningSolution>::HAS_CONSTRAINT_WEIGHTS);

fn main() {
    let mut weights = ConstraintWeights::new();
    weights.set("Unassigned task", 0);
    let plan = Plan {
        workers: Vec::new(),
        tasks: Vec::new(),
        weights: weights.clone(),
        score: None,
    };

    let _: Option<&ConstraintWeights> = plan.constraint_weights();
    let _: Result<(), solverforge::ConstraintWeightError> =
        Plan::validate_constraint_weights(&weights);
}
