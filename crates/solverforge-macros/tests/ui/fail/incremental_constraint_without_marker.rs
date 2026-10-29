// A hand-written constraint must also implement `SingleConstraint`.

use solverforge::prelude::*;

struct Plan;

struct Count {
    constraint_ref: ConstraintRef,
}

impl IncrementalConstraint<Plan, SoftScore> for Count {
    fn evaluate(&self, _: &Plan) -> SoftScore {
        SoftScore::ZERO
    }

    fn match_count(&self, _: &Plan) -> usize {
        0
    }

    fn initialize(&mut self, _: &Plan) -> SoftScore {
        SoftScore::ZERO
    }

    fn on_insert(&mut self, _: &Plan, _: usize, _: usize) -> SoftScore {
        SoftScore::ZERO
    }

    fn on_retract(&mut self, _: &Plan, _: usize, _: usize) -> SoftScore {
        SoftScore::ZERO
    }

    fn reset(&mut self) {}

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }
}

fn main() {}
