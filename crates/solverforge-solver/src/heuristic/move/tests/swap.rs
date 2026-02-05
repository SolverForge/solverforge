//! Tests for SwapMove operations.

use super::*;

#[derive(Clone, Debug)]
struct Task {
    id: usize,
    priority: Option<i32>,
}

#[derive(Clone, Debug)]
struct TaskSolution {
    tasks: Vec<Task>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for TaskSolution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
    &s.tasks
}

fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
    &mut s.tasks
}

fn get_priority(s: &TaskSolution, idx: usize) -> Option<i32> {
    s.tasks.get(idx).and_then(|t| t.priority)
}

fn set_priority(s: &mut TaskSolution, idx: usize, v: Option<i32>) {
    if let Some(task) = s.tasks.get_mut(idx) {
        task.priority = v;
    }
}

fn create_director(
    tasks: Vec<Task>,
) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
    let solution = TaskSolution { tasks, score: None };

    let extractor = Box::new(TypedEntityExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);

    let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
        .with_entity(entity_desc);

    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn test_swap_move_do_and_undo() {
    let tasks = vec![
        Task {
            id: 0,
            priority: Some(1),
        },
        Task {
            id: 1,
            priority: Some(5),
        },
    ];
    let mut director = create_director(tasks);

    let m = SwapMove::<TaskSolution, i32>::new(0, 1, get_priority, set_priority, "priority", 0);
    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        assert_eq!(get_priority(recording.working_solution(), 0), Some(5));
        assert_eq!(get_priority(recording.working_solution(), 1), Some(1));

        recording.undo_changes();
    }

    assert_eq!(get_priority(director.working_solution(), 0), Some(1));
    assert_eq!(get_priority(director.working_solution(), 1), Some(5));

    let solution = director.working_solution();
    assert_eq!(solution.tasks[0].id, 0);
    assert_eq!(solution.tasks[1].id, 1);
}

#[test]
fn test_swap_same_value_not_doable() {
    let tasks = vec![
        Task {
            id: 0,
            priority: Some(5),
        },
        Task {
            id: 1,
            priority: Some(5),
        },
    ];
    let director = create_director(tasks);

    let m = SwapMove::<TaskSolution, i32>::new(0, 1, get_priority, set_priority, "priority", 0);
    assert!(
        !m.is_doable(&director),
        "swapping same values should not be doable"
    );
}

#[test]
fn test_swap_self_not_doable() {
    let tasks = vec![Task {
        id: 0,
        priority: Some(1),
    }];
    let director = create_director(tasks);

    let m = SwapMove::<TaskSolution, i32>::new(0, 0, get_priority, set_priority, "priority", 0);
    assert!(!m.is_doable(&director), "self-swap should not be doable");
}

#[test]
fn test_swap_entity_indices() {
    let m = SwapMove::<TaskSolution, i32>::new(2, 5, get_priority, set_priority, "priority", 0);
    assert_eq!(m.entity_indices(), &[2, 5]);
}
