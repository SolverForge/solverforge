// Solve-start validation of a solution's runtime constraint weights.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{ConstraintSet, Director};

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

/* Rejects constraint weights on the director's working solution after a
problem change rewrote it: a change may install weights the solve-start check
never saw.
*/
pub(crate) fn validate_director_constraint_weights<S, D>(director: &D) -> RuntimeBuildResult<()>
where
    S: PlanningSolution,
    D: Director<S>,
{
    let Some(weights) = director.working_solution().constraint_weights() else {
        return Ok(());
    };
    let metadata = director.constraint_metadata();
    weights
        .validate(metadata.iter().map(|entry| entry.constraint_ref))
        .map_err(|error| RuntimeBuildError::ConstraintWeights {
            message: error.to_string(),
        })
}
