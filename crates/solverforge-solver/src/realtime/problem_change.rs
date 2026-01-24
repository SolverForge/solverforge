//! Problem change trait for real-time planning.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

/// A change to the problem that can be applied during solving.
///
/// Problem changes allow modifying the solution while the solver is running.
/// Changes are queued and processed at step boundaries to maintain consistency.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `C` - The constraint set type
///
/// # Implementation Notes
///
/// When implementing `ProblemChange`:
/// - Use `score_director.working_solution_mut()` to access and modify the solution
/// - Call `score_director.reset()` after major modifications to recalculate scores
/// - Changes should be idempotent when possible
/// - Avoid holding references to entities across changes
///
/// # Example
///
/// ```
/// use solverforge_solver::realtime::ProblemChange;
/// use solverforge_scoring::ScoreDirector;
/// use solverforge_scoring::api::constraint_set::ConstraintSet;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::{Score, SimpleScore};
///
/// #[derive(Clone, Debug)]
/// struct Employee { id: usize, shift: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Schedule {
///     employees: Vec<Employee>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for Schedule {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// /// Adds a new employee to the schedule.
/// #[derive(Debug)]
/// struct AddEmployee {
///     employee_id: usize,
/// }
///
/// impl<C> ProblemChange<Schedule, C> for AddEmployee
/// where
///     C: ConstraintSet<Schedule, SimpleScore>,
/// {
///     fn apply(&self, score_director: &mut ScoreDirector<Schedule, C>) {
///         // Add the new employee
///         score_director.working_solution_mut().employees.push(Employee {
///             id: self.employee_id,
///             shift: None,
///         });
///
///         // Reset to recalculate score
///         score_director.reset();
///     }
/// }
///
/// /// Removes an employee from the schedule.
/// #[derive(Debug)]
/// struct RemoveEmployee {
///     employee_id: usize,
/// }
///
/// impl<C> ProblemChange<Schedule, C> for RemoveEmployee
/// where
///     C: ConstraintSet<Schedule, SimpleScore>,
/// {
///     fn apply(&self, score_director: &mut ScoreDirector<Schedule, C>) {
///         // Remove the employee
///         let id = self.employee_id;
///         score_director.working_solution_mut().employees.retain(|e| e.id != id);
///
///         // Reset to recalculate score
///         score_director.reset();
///     }
/// }
/// ```
pub trait ProblemChange<S, C>: Send + Debug
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Applies this change to the working solution.
    ///
    /// This method is called by the solver at a safe point (between steps).
    /// Access the working solution via `score_director.working_solution_mut()`.
    ///
    /// After making changes, call `score_director.reset()` to recalculate
    /// scores from scratch.
    fn apply(&self, score_director: &mut ScoreDirector<S, C>);
}

/// A boxed problem change for type-erased storage.
pub type BoxedProblemChange<S, C> = Box<dyn ProblemChange<S, C>>;

/// A problem change implemented as a closure.
///
/// This is a convenience wrapper for simple changes that don't need
/// a dedicated struct.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `C` - The constraint set type
/// * `F` - The closure type
///
/// # Example
///
/// ```
/// use solverforge_solver::realtime::ClosureProblemChange;
/// use solverforge_scoring::ScoreDirector;
/// use solverforge_scoring::api::constraint_set::ConstraintSet;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::{Score, SimpleScore};
///
/// #[derive(Clone, Debug)]
/// struct Task { id: usize, done: bool }
///
/// #[derive(Clone, Debug)]
/// struct Solution {
///     tasks: Vec<Task>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Mark task 0 as done (with explicit constraint set type)
/// fn create_change<C>() -> ClosureProblemChange<Solution, C, impl Fn(&mut ScoreDirector<Solution, C>) + Send>
/// where
///     C: ConstraintSet<Solution, SimpleScore>,
/// {
///     ClosureProblemChange::new("mark_task_done", |sd: &mut ScoreDirector<Solution, C>| {
///         if let Some(task) = sd.working_solution_mut().tasks.get_mut(0) {
///             task.done = true;
///         }
///         sd.reset();
///     })
/// }
/// ```
pub struct ClosureProblemChange<S, C, F>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    F: Fn(&mut ScoreDirector<S, C>) + Send,
{
    name: &'static str,
    change_fn: F,
    _phantom: std::marker::PhantomData<(S, C)>,
}

impl<S, C, F> ClosureProblemChange<S, C, F>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    F: Fn(&mut ScoreDirector<S, C>) + Send,
{
    /// Creates a new closure-based problem change.
    ///
    /// # Arguments
    /// * `name` - A descriptive name for debugging
    /// * `change_fn` - The closure that applies the change
    pub fn new(name: &'static str, change_fn: F) -> Self {
        Self {
            name,
            change_fn,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S, C, F> Debug for ClosureProblemChange<S, C, F>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    F: Fn(&mut ScoreDirector<S, C>) + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosureProblemChange")
            .field("name", &self.name)
            .finish()
    }
}

impl<S, C, F> ProblemChange<S, C> for ClosureProblemChange<S, C, F>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    F: Fn(&mut ScoreDirector<S, C>) + Send,
{
    fn apply(&self, score_director: &mut ScoreDirector<S, C>) {
        (self.change_fn)(score_director);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Debug)]
    struct Task {
        id: usize,
    }

    #[derive(Clone, Debug)]
    struct TaskSchedule {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TaskSchedule {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn create_director(tasks: Vec<Task>) -> ScoreDirector<TaskSchedule, ()> {
        let solution = TaskSchedule { tasks, score: None };
        ScoreDirector::new(solution, ())
    }

    #[derive(Debug)]
    struct AddTask {
        id: usize,
    }

    impl<C> ProblemChange<TaskSchedule, C> for AddTask
    where
        C: ConstraintSet<TaskSchedule, SimpleScore>,
    {
        fn apply(&self, score_director: &mut ScoreDirector<TaskSchedule, C>) {
            score_director
                .working_solution_mut()
                .tasks
                .push(Task { id: self.id });
            score_director.reset();
        }
    }

    #[test]
    fn struct_problem_change() {
        let mut director = create_director(vec![Task { id: 0 }]);

        let change = AddTask { id: 1 };
        change.apply(&mut director);

        assert_eq!(director.working_solution().tasks.len(), 2);
        assert_eq!(director.working_solution().tasks[1].id, 1);
    }

    #[test]
    fn closure_problem_change() {
        let mut director = create_director(vec![Task { id: 0 }]);

        let change: ClosureProblemChange<TaskSchedule, (), _> =
            ClosureProblemChange::new("remove_all", |sd| {
                sd.working_solution_mut().tasks.clear();
                sd.reset();
            });

        change.apply(&mut director);

        assert!(director.working_solution().tasks.is_empty());
    }
}
