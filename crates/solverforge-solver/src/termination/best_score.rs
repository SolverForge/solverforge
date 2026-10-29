// Score-based termination conditions.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use super::Termination;
use crate::scope::ProgressCallback;
use crate::scope::SolverScope;

/* Terminates when the best score reaches or exceeds a target score.

This is useful when you know what score you're aiming for (e.g., a perfect
score of 0 for constraint satisfaction problems).

# Example

```
use solverforge_solver::termination::BestScoreTermination;
use solverforge_core::score::SoftScore;

// Terminate when score reaches 0 (no constraint violations)
let term: BestScoreTermination<SoftScore> = BestScoreTermination::new(SoftScore::of(0));
```
*/
#[derive(Debug, Clone)]
pub(crate) struct BestScoreTermination<Sc: Score> {
    target_score: Sc,
}

impl<Sc: Score> BestScoreTermination<Sc> {
    pub(crate) fn new(target_score: Sc) -> Self {
        Self { target_score }
    }
}

impl<S, D, BestCb, Sc> Termination<S, D, BestCb> for BestScoreTermination<Sc>
where
    S: PlanningSolution<Score = Sc>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    Sc: Score,
{
    fn is_terminated(&mut self, solver_scope: &SolverScope<S, D, BestCb>) -> bool {
        solver_scope
            .best_score()
            .map(|score| *score >= self.target_score)
            .unwrap_or(false)
    }

    fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        solver_scope.install_inphase_best_score_limit(self.target_score);
    }
}
