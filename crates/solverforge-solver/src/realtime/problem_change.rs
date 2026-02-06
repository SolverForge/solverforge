//! Problem change trait for real-time planning.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

/// A change to the problem that can be applied during solving.
///
/// Problem changes allow modifying the solution while the solver is running.
/// Changes are queued and processed at step boundaries to maintain consistency.
///
/// # Implementation Notes
///
/// When implementing `ProblemChange`:
/// - Use `score_director.working_solution_mut()` to access and modify the solution
/// - Call `score_director.trigger_variable_listeners()` after modifications
/// - Changes should be idempotent when possible
/// - Avoid holding references to entities across changes
///
/// # Example
///
/// ```
/// use solverforge_solver::realtime::ProblemChange;
/// use solverforge_scoring::ScoreDirector;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
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
/// impl ProblemChange<Schedule> for AddEmployee {
///     fn apply(&self, score_director: &mut dyn ScoreDirector<Schedule>) {
///         // Add the new employee
///         score_director.working_solution_mut().employees.push(Employee {
///             id: self.employee_id,
///             shift: None,
///         });
///
///         // Notify the solver of the change
///         score_director.trigger_variable_listeners();
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
///     fn apply(&self, score_director: &mut dyn ScoreDirector<Schedule>) {
///         // Remove the employee
///         let id = self.employee_id;
///         score_director.working_solution_mut().employees.retain(|e| e.id != id);
///
///         // Notify the solver of the change
///         score_director.trigger_variable_listeners();
///     }
/// }
/// ```
pub trait ProblemChange<S: PlanningSolution>: Send + Debug {
    /// Applies this change to the working solution.
    ///
    /// This method is called by the solver at a safe point (between steps).
    /// Access the working solution via `score_director.working_solution_mut()`.
    ///
    /// After making changes, call `score_director.trigger_variable_listeners()`
    /// to ensure shadow variables and constraints are updated.
    fn apply(&self, score_director: &mut dyn ScoreDirector<S>);
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
/// use solverforge_scoring::ScoreDirector;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
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
/// // Mark task 0 as done
/// let change = ClosureProblemChange::<Solution, _>::new("mark_task_done", |sd| {
///     if let Some(task) = sd.working_solution_mut().tasks.get_mut(0) {
///         task.done = true;
///     }
///     sd.trigger_variable_listeners();
/// });
/// ```
pub struct ClosureProblemChange<S: PlanningSolution, F>
where
    F: Fn(&mut dyn ScoreDirector<S>) + Send,
{
    name: &'static str,
    change_fn: F,
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S, F> ClosureProblemChange<S, F>
where
    S: PlanningSolution,
    F: Fn(&mut dyn ScoreDirector<S>) + Send,
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
    F: Fn(&mut dyn ScoreDirector<S>) + Send,
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
    F: Fn(&mut dyn ScoreDirector<S>) + Send,
{
    fn apply(&self, score_director: &mut dyn ScoreDirector<S>) {
        (self.change_fn)(score_director);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

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

    fn get_tasks(s: &TaskSchedule) -> &Vec<Task> {
        &s.tasks
    }
    fn get_tasks_mut(s: &mut TaskSchedule) -> &mut Vec<Task> {
        &mut s.tasks
    }

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<TaskSchedule, impl Fn(&TaskSchedule) -> SimpleScore> {
        let solution = TaskSchedule { tasks, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            get_tasks,
            get_tasks_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TaskSchedule", TypeId::of::<TaskSchedule>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[derive(Debug)]
    struct AddTask {
        id: usize,
    }

    impl ProblemChange<TaskSchedule> for AddTask {
        fn apply(&self, score_director: &mut dyn ScoreDirector<TaskSchedule>) {
            score_director
                .working_solution_mut()
                .tasks
                .push(Task { id: self.id });
            score_director.trigger_variable_listeners();
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

        let change = ClosureProblemChange::<TaskSchedule, _>::new("remove_all", |sd| {
            sd.working_solution_mut().tasks.clear();
            sd.trigger_variable_listeners();
        });

        change.apply(&mut director);

        assert!(director.working_solution().tasks.is_empty());
    }
}
