//! Problem change trait for real-time planning.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

/// A change to the problem that can be applied during solving.
///
/// The solver takes ownership of a submitted change and applies it once at a
/// safe point, so a change can move its payload (a new entity, a replacement
/// fact) into the working solution without cloning it.
///
/// # Implementation Notes
///
/// - Use `score_director.working_solution_mut()` to access and modify the
///   solution. The solver fully reinitializes scoring after the change, so a
///   change does not call `before_variable_changed`/`after_variable_changed`.
/// - Keep every index-valued planning variable and fact reference valid: a
///   change that removes a fact must also clear or remap the entities that
///   point at it.
///
/// # Example
///
/// ```
/// use solverforge_solver::ProblemChange;
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
/// /// Adds a new employee to the schedule.
/// #[derive(Debug)]
/// struct AddEmployee {
///     employee: Employee,
/// }
///
/// impl ProblemChange<Schedule> for AddEmployee {
///     fn apply(self: Box<Self>, score_director: &mut dyn Director<Schedule>) {
///         score_director.working_solution_mut().employees.push(self.employee);
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
///     fn apply(self: Box<Self>, score_director: &mut dyn Director<Schedule>) {
///         let id = self.employee_id;
///         score_director.working_solution_mut().employees.retain(|e| e.id != id);
///     }
/// }
/// ```
pub trait ProblemChange<S: PlanningSolution>: Send + Debug {
    /// Applies this change to the working solution, consuming it.
    fn apply(self: Box<Self>, score_director: &mut dyn Director<S>);
}

/// A problem change implemented as a one-shot closure.
///
/// This is a convenience wrapper for simple changes that don't need
/// a dedicated struct.
///
/// # Example
///
/// ```
/// use solverforge_solver::ClosureProblemChange;
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
/// // Add a task, moving it into the solution.
/// let task = Task { id: 7, done: false };
/// let change = ClosureProblemChange::<Solution, _>::new("add_task", move |sd| {
///     sd.working_solution_mut().tasks.push(task);
/// });
/// ```
pub struct ClosureProblemChange<S: PlanningSolution, F>
where
    F: FnOnce(&mut dyn Director<S>) + Send,
{
    name: &'static str,
    change_fn: F,
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S, F> ClosureProblemChange<S, F>
where
    S: PlanningSolution,
    F: FnOnce(&mut dyn Director<S>) + Send,
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
    F: FnOnce(&mut dyn Director<S>) + Send,
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
    F: FnOnce(&mut dyn Director<S>) + Send,
{
    fn apply(self: Box<Self>, score_director: &mut dyn Director<S>) {
        (self.change_fn)(score_director);
    }
}

#[cfg(test)]
#[path = "problem_change_tests.rs"]
mod tests;
