// Problem change trait for real-time planning.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

/// A change to the problem that can be applied during solving.
///
/// Problem changes allow modifying the solution while the solver is running.
/// Changes are queued and processed at step boundaries to maintain consistency.
/// The solver applies each change through its committed mutation boundary so
/// optional-construction frontier revisions stay coherent.
///
/// # Implementation Notes
///
/// When implementing `ProblemChange`:
/// - Use `score_director.working_solution_mut()` to access and modify the solution
/// - Changes should be idempotent when possible
/// - Avoid holding references to entities across changes
///
/// # Example
///
/// ```
/// use solverforge_solver::realtime::ProblemChange;
/// use solverforge_scoring::Director;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Employee { id: usize, shift: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Schedule {
///     employees: Vec<Employee>,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for Schedule {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
// /// Adds a new employee to the schedule.
/// #[derive(Debug)]
/// struct AddEmployee {
///     employee_id: usize,
/// }
///
/// impl ProblemChange<Schedule> for AddEmployee {
///     fn apply(&self, score_director: &mut dyn Director<Schedule>) {
///         // Add the new employee
///         score_director.working_solution_mut().employees.push(Employee {
///             id: self.employee_id,
///             shift: None,
///         });
///
///     }
/// }
///
/// /// Removes an employee from the schedule.
/// #[derive(Debug)]
/// struct RemoveEmployee {
///     employee_id: usize,
/// }
///
/// impl ProblemChange<Schedule> for RemoveEmployee {
///     fn apply(&self, score_director: &mut dyn Director<Schedule>) {
///         // Remove the employee
///         let id = self.employee_id;
///         score_director.working_solution_mut().employees.retain(|e| e.id != id);
///     }
/// }
/// ```
pub trait ProblemChange<S: PlanningSolution>: Send + Debug {
    /* Applies this change to the working solution.

    This method is called by the solver at a safe point (between steps).
    Access the working solution via `score_director.working_solution_mut()`.

    */
    fn apply(&self, score_director: &mut dyn Director<S>);
}

/// A boxed problem change for type-erased storage.
pub type BoxedProblemChange<S> = Box<dyn ProblemChange<S>>;

/// A problem change implemented as a closure.
///
/// This is a convenience wrapper for simple changes that don't need
/// a dedicated struct.
///
/// # Example
///
/// ```
/// use solverforge_solver::realtime::ClosureProblemChange;
/// use solverforge_scoring::Director;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { id: usize, done: bool }
///
/// #[derive(Clone, Debug)]
/// struct Solution {
///     tasks: Vec<Task>,
///     score: Option<SoftScore>,
/// }
///
/// impl PlanningSolution for Solution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Mark task 0 as done
/// let change = ClosureProblemChange::<Solution, _>::new("mark_task_done", |sd| {
///     if let Some(task) = sd.working_solution_mut().tasks.get_mut(0) {
///         task.done = true;
///     }
/// });
/// ```
pub struct ClosureProblemChange<S: PlanningSolution, F>
where
    F: Fn(&mut dyn Director<S>) + Send,
{
    name: &'static str,
    change_fn: F,
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S, F> ClosureProblemChange<S, F>
where
    S: PlanningSolution,
    F: Fn(&mut dyn Director<S>) + Send,
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

impl<S, F> Debug for ClosureProblemChange<S, F>
where
    S: PlanningSolution,
    F: Fn(&mut dyn Director<S>) + Send,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClosureProblemChange")
            .field("name", &self.name)
            .finish()
    }
}

impl<S, F> ProblemChange<S> for ClosureProblemChange<S, F>
where
    S: PlanningSolution,
    F: Fn(&mut dyn Director<S>) + Send,
{
    fn apply(&self, score_director: &mut dyn Director<S>) {
        (self.change_fn)(score_director);
    }
}

#[cfg(test)]
#[path = "problem_change_tests.rs"]
mod tests;
