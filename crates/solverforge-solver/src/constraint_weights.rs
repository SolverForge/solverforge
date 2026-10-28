// Solve-start validation of a solution's runtime constraint weights.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ConstraintSet;

use crate::runtime_build_error::{RuntimeBuildError, RuntimeBuildResult};

/* Rejects constraint weights that name no constraint of `constraints` or
exceed the multiplier cap, before any solving work starts.
*/
pub(crate) fn validate_constraint_weights<S, C>(
    solution: &S,
    constraints: &C,
) -> RuntimeBuildResult<()>
where
    S: PlanningSolution,
    C: ConstraintSet<S, S::Score>,
{
    let Some(weights) = solution.constraint_weights() else {
        return Ok(());
    };
    let metadata = constraints.constraint_metadata();
    weights
        .validate(metadata.iter().map(|entry| entry.constraint_ref))
        .map_err(|error| RuntimeBuildError::ConstraintWeights {
            message: error.to_string(),
        })
}
