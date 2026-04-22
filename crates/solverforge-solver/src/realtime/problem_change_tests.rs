use super::*;
use crate::phase::construction::{ConstructionListElementId, ConstructionSlotId};
use crate::scope::SolverScope;
use solverforge_core::domain::{EntityCollectionExtractor, EntityDescriptor, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Task {
    id: usize,
}

#[derive(Clone, Debug)]
struct TaskSchedule {
    tasks: Vec<Task>,
    score: Option<SoftScore>,
}

impl PlanningSolution for TaskSchedule {
    type Score = SoftScore;

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

fn create_director(tasks: Vec<Task>) -> ScoreDirector<TaskSchedule, ()> {
    let solution = TaskSchedule { tasks, score: None };
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("TaskSchedule", TypeId::of::<TaskSchedule>())
        .with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.tasks.len())
}

#[derive(Debug)]
struct AddTask {
    id: usize,
}

impl ProblemChange<TaskSchedule> for AddTask {
    fn apply(&self, score_director: &mut dyn Director<TaskSchedule>) {
        score_director
            .working_solution_mut()
            .tasks
            .push(Task { id: self.id });
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
    });

    change.apply(&mut director);

    assert!(director.working_solution().tasks.is_empty());
}

#[test]
fn problem_change_apply_through_solver_scope_invalidates_frontier_revision() {
    let director = create_director(vec![Task { id: 0 }]);
    let mut scope = SolverScope::new(director);
    scope.start_solving();

    let slot_id = ConstructionSlotId::new(0, 0);
    let element_id = ConstructionListElementId::new(0, 0);
    scope.mark_scalar_slot_completed(slot_id);
    scope.mark_list_element_completed(element_id);
    let initial_revision = scope.solution_revision();

    let change = AddTask { id: 1 };
    scope.mutate(|score_director| change.apply(score_director));

    assert_eq!(scope.solution_revision(), initial_revision + 1);
    assert!(!scope.is_scalar_slot_completed(slot_id));
    assert!(!scope.is_list_element_completed(element_id));
    assert_eq!(scope.working_solution().tasks.len(), 2);
    assert_eq!(scope.working_solution().tasks[1].id, 1);
}
