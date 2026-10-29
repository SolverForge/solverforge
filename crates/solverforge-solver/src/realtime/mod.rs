/* Real-time planning support.

Problem changes are submitted to a running job with
`SolverManager::add_problem_change`. The configured runtime applies queued
changes in submission order at its next safe point, rescores the changed
solution from scratch, and restarts its phase sequence so construction
assigns any new or unassigned planning work before local search resumes.

Typical changes:
- Add new entities (e.g., new orders, tasks, employees)
- Remove entities (e.g., cancelled orders)
- Update entity properties (e.g., deadline changes)
- Modify problem facts

# Example

```
use solverforge_solver::realtime::ProblemChange;
use solverforge_scoring::Director;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

#[derive(Clone, Debug)]
struct Task { id: usize, priority: Option<i32> }

#[derive(Clone, Debug)]
struct Schedule {
tasks: Vec<Task>,
score: Option<SoftScore>,
}

impl PlanningSolution for Schedule {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

// Create a problem change that adds a new task
#[derive(Debug)]
struct AddTask { task: Task }

impl ProblemChange<Schedule> for AddTask {
fn apply(self: Box<Self>, score_director: &mut dyn Director<Schedule>) {
score_director.working_solution_mut().tasks.push(self.task);
}
}
```
*/

pub(crate) mod problem_change;

pub(crate) use problem_change::ProblemChange;
