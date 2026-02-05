//! Tests for ChangeMove operations.

use super::*;

#[derive(Clone, Debug, PartialEq)]
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

fn get_priority(s: &TaskSolution, i: usize) -> Option<i32> {
    s.tasks.get(i).and_then(|t| t.priority)
}

fn set_priority(s: &mut TaskSolution, i: usize, v: Option<i32>) {
    if let Some(task) = s.tasks.get_mut(i) {
        task.priority = v;
    }
}

fn create_director(
    tasks: Vec<Task>,
) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
    let solution = TaskSolution { tasks, score: None };
    let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>());
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn test_change_move_is_doable() {
    let tasks = vec![
        Task {
            id: 0,
            priority: Some(1),
        },
        Task {
            id: 1,
            priority: Some(2),
        },
    ];
    let director = create_director(tasks);

    // Different value - doable
    let m = ChangeMove::<_, i32>::new(0, Some(5), get_priority, set_priority, "priority", 0);
    assert!(m.is_doable(&director));

    // Same value - not doable
    let m = ChangeMove::<_, i32>::new(0, Some(1), get_priority, set_priority, "priority", 0);
    assert!(!m.is_doable(&director));
}

#[test]
fn test_change_move_do_move() {
    let tasks = vec![Task {
        id: 0,
        priority: Some(1),
    }];
    let mut director = create_director(tasks);

    let m = ChangeMove::<_, i32>::new(0, Some(5), get_priority, set_priority, "priority", 0);
    m.do_move(&mut director);

    let val = get_priority(director.working_solution(), 0);
    assert_eq!(val, Some(5));
}

#[test]
fn test_change_move_to_none() {
    let tasks = vec![Task {
        id: 0,
        priority: Some(5),
    }];
    let mut director = create_director(tasks);

    let m = ChangeMove::<_, i32>::new(0, None, get_priority, set_priority, "priority", 0);
    assert!(m.is_doable(&director));

    m.do_move(&mut director);

    let val = get_priority(director.working_solution(), 0);
    assert_eq!(val, None);
}

#[test]
fn test_change_move_entity_indices() {
    let m =
        ChangeMove::<TaskSolution, i32>::new(3, Some(5), get_priority, set_priority, "priority", 0);
    assert_eq!(m.entity_indices(), &[3]);
}

#[test]
fn test_change_move_copy() {
    let m1 =
        ChangeMove::<TaskSolution, i32>::new(0, Some(5), get_priority, set_priority, "priority", 0);
    let m2 = m1;
    assert_eq!(m1.entity_index(), m2.entity_index());
    assert_eq!(m1.to_value(), m2.to_value());
}
