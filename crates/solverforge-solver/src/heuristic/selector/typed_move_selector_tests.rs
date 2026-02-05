//! Tests for typed move selectors.

use super::*;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
use std::any::TypeId;

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

// Typed getter - zero erasure
fn get_priority(s: &TaskSolution, idx: usize) -> Option<i32> {
    s.tasks.get(idx).and_then(|t| t.priority)
}

// Typed setter - zero erasure
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
fn test_change_move_selector() {
    let director = create_director(vec![
        Task {
            id: 0,
            priority: Some(1),
        },
        Task {
            id: 1,
            priority: Some(2),
        },
        Task {
            id: 2,
            priority: Some(3),
        },
    ]);

    // Verify entity IDs
    let solution = director.working_solution();
    assert_eq!(solution.tasks[0].id, 0);
    assert_eq!(solution.tasks[1].id, 1);
    assert_eq!(solution.tasks[2].id, 2);

    let selector =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20, 30]);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    // 3 entities * 3 values = 9 moves
    assert_eq!(moves.len(), 9);
    assert_eq!(selector.size(&director), 9);

    // Verify first move structure
    let first = &moves[0];
    assert_eq!(first.entity_index(), 0);
    assert_eq!(first.to_value(), Some(&10));
}

#[test]
fn test_swap_move_selector() {
    let director = create_director(vec![
        Task {
            id: 0,
            priority: Some(1),
        },
        Task {
            id: 1,
            priority: Some(2),
        },
        Task {
            id: 2,
            priority: Some(3),
        },
        Task {
            id: 3,
            priority: Some(4),
        },
    ]);

    let selector = SwapMoveSelector::simple(get_priority, set_priority, 0, "priority");

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    // 4 entities: 4*3/2 = 6 pairs
    assert_eq!(moves.len(), 6);
    assert_eq!(selector.size(&director), 6);

    // Verify first swap
    let first = &moves[0];
    assert_eq!(first.left_entity_index(), 0);
    assert_eq!(first.right_entity_index(), 1);
}

#[test]
fn test_change_do_and_undo() {
    let mut director = create_director(vec![Task {
        id: 0,
        priority: Some(1),
    }]);

    let selector = ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![99]);

    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(moves.len(), 1);

    let m = &moves[0];
    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        // Verify change using typed getter - zero erasure
        let val = get_priority(recording.working_solution(), 0);
        assert_eq!(val, Some(99));

        // Undo
        recording.undo_changes();
    }

    // Verify restored using typed getter
    let val = get_priority(director.working_solution(), 0);
    assert_eq!(val, Some(1));
}

#[test]
fn test_swap_do_and_undo() {
    let mut director = create_director(vec![
        Task {
            id: 0,
            priority: Some(10),
        },
        Task {
            id: 1,
            priority: Some(20),
        },
    ]);

    let selector = SwapMoveSelector::simple(get_priority, set_priority, 0, "priority");

    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(moves.len(), 1);

    let m = &moves[0];
    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        // Verify swap using typed getter
        let val0 = get_priority(recording.working_solution(), 0);
        let val1 = get_priority(recording.working_solution(), 1);
        assert_eq!(val0, Some(20));
        assert_eq!(val1, Some(10));

        // Undo
        recording.undo_changes();
    }

    // Verify restored using typed getter
    let val0 = get_priority(director.working_solution(), 0);
    let val1 = get_priority(director.working_solution(), 1);
    assert_eq!(val0, Some(10));
    assert_eq!(val1, Some(20));
}
