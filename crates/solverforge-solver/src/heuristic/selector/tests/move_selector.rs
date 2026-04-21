// Tests for typed move selectors.

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, RecordingDirector, ScoreDirector};
use std::any::TypeId;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    ChangeMoveSelector, MoveSelector, SwapMoveSelector,
};

#[derive(Clone, Debug)]
struct Task {
    id: usize,
    priority: Option<i32>,
}

#[derive(Clone, Debug)]
struct TaskSolution {
    tasks: Vec<Task>,
    score: Option<SoftScore>,
}

impl PlanningSolution for TaskSolution {
    type Score = SoftScore;
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

fn create_director(tasks: Vec<Task>) -> ScoreDirector<TaskSolution, ()> {
    let solution = TaskSolution { tasks, score: None };

    let extractor = Box::new(EntityCollectionExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);

    let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
        .with_entity(entity_desc);

    ScoreDirector::simple(solution, descriptor, |s, _| s.tasks.len())
}

#[derive(Clone, Debug)]
struct CountedTask {
    value: Option<CountedValue>,
}

#[derive(Clone, Debug)]
struct CountedSolution {
    tasks: Vec<CountedTask>,
    score: Option<SoftScore>,
}

impl PlanningSolution for CountedSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Debug)]
struct CountedValue {
    id: usize,
    cloned: Arc<AtomicUsize>,
}

impl Clone for CountedValue {
    fn clone(&self) -> Self {
        self.cloned.fetch_add(1, Ordering::SeqCst);
        Self {
            id: self.id,
            cloned: Arc::clone(&self.cloned),
        }
    }
}

impl PartialEq for CountedValue {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

fn get_counted_tasks(s: &CountedSolution) -> &Vec<CountedTask> {
    &s.tasks
}

fn get_counted_tasks_mut(s: &mut CountedSolution) -> &mut Vec<CountedTask> {
    &mut s.tasks
}

fn get_counted_value(s: &CountedSolution, idx: usize) -> Option<CountedValue> {
    s.tasks.get(idx).and_then(|task| task.value.clone())
}

fn set_counted_value(s: &mut CountedSolution, idx: usize, v: Option<CountedValue>) {
    if let Some(task) = s.tasks.get_mut(idx) {
        task.value = v;
    }
}

fn create_counted_director(tasks: Vec<CountedTask>) -> ScoreDirector<CountedSolution, ()> {
    let solution = CountedSolution { tasks, score: None };

    let extractor = Box::new(EntityCollectionExtractor::new(
        "CountedTask",
        "tasks",
        get_counted_tasks,
        get_counted_tasks_mut,
    ));
    let entity_desc = EntityDescriptor::new("CountedTask", TypeId::of::<CountedTask>(), "tasks")
        .with_extractor(extractor);

    let descriptor = SolutionDescriptor::new("CountedSolution", TypeId::of::<CountedSolution>())
        .with_entity(entity_desc);

    ScoreDirector::simple(solution, descriptor, |s, _| s.tasks.len())
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
fn change_selector_emits_single_to_none_move_for_assigned_entities_when_enabled() {
    let director = create_director(vec![
        Task {
            id: 0,
            priority: Some(1),
        },
        Task {
            id: 1,
            priority: None,
        },
    ]);

    let selector =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20])
            .with_allows_unassigned(true);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 5);
    assert_eq!(moves.len(), 5);
    assert_eq!(
        moves
            .iter()
            .filter(|mov| mov.entity_index() == 0)
            .map(|mov| mov.to_value().copied())
            .collect::<Vec<_>>(),
        vec![Some(10), Some(20), None]
    );
    assert_eq!(
        moves
            .iter()
            .filter(|mov| mov.entity_index() == 1)
            .map(|mov| mov.to_value().copied())
            .collect::<Vec<_>>(),
        vec![Some(10), Some(20)]
    );
}

#[test]
fn change_selector_does_not_emit_to_none_without_unassigned_support() {
    let director = create_director(vec![Task {
        id: 0,
        priority: Some(1),
    }]);

    let selector =
        ChangeMoveSelector::simple(get_priority, set_priority, 0, "priority", vec![10, 20]);

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 2);
    assert!(moves.iter().all(|mov| mov.to_value().is_some()));
}

#[test]
fn change_selector_clones_values_as_cursor_advances() {
    let director = create_counted_director(vec![CountedTask { value: None }]);
    let cloned = Arc::new(AtomicUsize::new(0));
    let values = (0..10)
        .map(|id| CountedValue {
            id,
            cloned: Arc::clone(&cloned),
        })
        .collect();
    let selector =
        ChangeMoveSelector::simple(get_counted_value, set_counted_value, 0, "counted", values);

    let mut cursor = selector.open_cursor(&director);

    assert_eq!(cloned.load(Ordering::SeqCst), 0);

    let first = cursor.next().expect("first move should be available");
    assert_eq!(first.entity_index(), 0);
    assert_eq!(first.to_value().map(|value| value.id), Some(0));
    assert_eq!(cloned.load(Ordering::SeqCst), 1);

    let next_two: Vec<_> = cursor.by_ref().take(2).collect();
    assert_eq!(next_two.len(), 2);
    assert_eq!(
        next_two
            .iter()
            .map(|mov| mov.to_value().map(|value| value.id))
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2)]
    );
    assert_eq!(cloned.load(Ordering::SeqCst), 3);
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
        let mut recording = RecordingDirector::new(&mut director);
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
        let mut recording = RecordingDirector::new(&mut director);
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
