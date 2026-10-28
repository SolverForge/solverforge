/* Per-constraint score analysis on the director.

Every result reflects the bound constraint weights: solution types with
`PlanningSolution::HAS_CONSTRAINT_WEIGHTS` report weighted scores, match
scores and multipliers whenever some multiplier differs from `1`.
*/

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::descriptor::DirectorDescriptor;
use super::incremental::ScoreDirector;
use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintResult, ConstraintSet};

impl<S, C, Desc> ScoreDirector<S, C, Desc>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    Desc: DirectorDescriptor<S, C>,
{
    // Evaluates every constraint of the working solution under the bound weights.
    pub fn constraint_results(&self) -> Vec<ConstraintResult<'_, S::Score>> {
        if self.weighted() {
            return self
                .constraints
                .evaluate_each_weighted(&self.working_solution, self.weights.multipliers());
        }
        self.constraints.evaluate_each(&self.working_solution)
    }

    /* Analyzes every constraint of the working solution, with per-match
    justifications, under the bound weights. */
    pub fn constraint_analyses(&self) -> Vec<ConstraintAnalysis<'_, S::Score>> {
        if self.weighted() {
            return self
                .constraints
                .evaluate_detailed_weighted(&self.working_solution, self.weights.multipliers());
        }
        self.constraints.evaluate_detailed(&self.working_solution)
    }

    /* Returns `(name, weight_multiplier, weighted score, match_count)` per
    constraint for score analysis. */
    pub fn constraint_match_totals(&self) -> Vec<(String, u32, S::Score, usize)> {
        self.constraint_results()
            .into_iter()
            .map(|r| {
                (
                    r.name.to_string(),
                    r.weight_multiplier,
                    r.score,
                    r.match_count,
                )
            })
            .collect()
    }
}
