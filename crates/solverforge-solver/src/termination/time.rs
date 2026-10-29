// Time-based termination.

use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::Termination;
use crate::scope::ProgressCallback;
use crate::scope::SolverScope;

/* Terminates after a time limit.

# Example

```
use std::time::Duration;
use solverforge_solver::termination::TimeTermination;

let term = TimeTermination::new(Duration::from_secs(30));
let term = TimeTermination::seconds(30);
let term = TimeTermination::millis(500);
```
*/
#[derive(Debug, Clone)]
pub(crate) struct TimeTermination {
    limit: Duration,
}

impl TimeTermination {
    pub(crate) fn new(limit: Duration) -> Self {
        Self { limit }
    }
}

impl<S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S>> Termination<S, D, BestCb>
    for TimeTermination
{
    fn is_terminated(&mut self, solver_scope: &SolverScope<S, D, BestCb>) -> bool {
        solver_scope.elapsed().is_some_and(|e| e >= self.limit)
    }
}
