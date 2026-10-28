/* Per-constraint score analysis on the director.

Every result reflects the director's weight mode: a `WeightedMode` director
reports weighted scores, match scores and bound multipliers; an
`UnweightedMode` director reports coded weights at multiplier `1`.
*/

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::descriptor::DirectorDescriptor;
use super::incremental::ScoreDirector;
use super::weight_mode::WeightMode;
use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintResult, ConstraintSet};

impl<S, C, Desc, M> ScoreDirector<S, C, Desc, M>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    Desc: DirectorDescriptor<S, C>,
    M: WeightMode,
{
    // Evaluates every constraint of the working solution under the director's weight mode.
    pub fn constraint_results(&self) -> Vec<ConstraintResult<'_, S::Score>> {
        self.weight_mode
            .evaluate_each(&self.constraints, &self.working_solution)
    }

    /* Analyzes every constraint of the working solution, with per-match
    justifications, under the director's weight mode. */
    pub fn constraint_analyses(&self) -> Vec<ConstraintAnalysis<'_, S::Score>> {
        self.weight_mode
            .evaluate_detailed(&self.constraints, &self.working_solution)
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
