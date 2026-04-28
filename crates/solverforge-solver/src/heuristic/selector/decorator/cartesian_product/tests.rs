use super::*;
use crate::heuristic::r#move::{ChangeMove, Move, ScalarMoveUnion};
use crate::heuristic::selector::move_selector::{
    collect_cursor_indices, ArenaMoveCursor, CandidateId, MoveCandidateRef, MoveCursor,
};
use crate::heuristic::selector::{ChangeMoveSelector, MoveSelector};
use crate::phase::localsearch::{Acceptor, TabuSearchAcceptor, TabuSearchPolicy};
use solverforge_core::domain::{EntityCollectionExtractor, EntityDescriptor, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Task {
    x: Option<i32>,
    y: Option<i32>,
    shadow_y_target: Option<i32>,
}

#[derive(Clone, Debug)]
struct Sol {
    tasks: Vec<Task>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Sol {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    fn update_entity_shadows(&mut self, descriptor_index: usize, entity_index: usize) {
        if descriptor_index != 0 {
            return;
        }

        if let Some(task) = self.tasks.get_mut(entity_index) {
            task.shadow_y_target = task.x.map(|value| value + 10);
        }
    }

    fn update_all_shadows(&mut self) {
        for entity_index in 0..self.tasks.len() {
            self.update_entity_shadows(0, entity_index);
        }
    }
}

fn get_tasks(s: &Sol) -> &Vec<Task> {
    &s.tasks
}

fn get_tasks_mut(s: &mut Sol) -> &mut Vec<Task> {
    &mut s.tasks
}

fn get_x(s: &Sol, i: usize, _variable_index: usize) -> Option<i32> {
    s.tasks.get(i).and_then(|t| t.x)
}

fn set_x(s: &mut Sol, i: usize, _variable_index: usize, v: Option<i32>) {
    if let Some(t) = s.tasks.get_mut(i) {
        t.x = v;
    }
}

fn get_y(s: &Sol, i: usize, _variable_index: usize) -> Option<i32> {
    s.tasks.get(i).and_then(|t| t.y)
}

fn set_y(s: &mut Sol, i: usize, _variable_index: usize, v: Option<i32>) {
    if let Some(t) = s.tasks.get_mut(i) {
        t.y = v;
    }
}

fn create_director(tasks: Vec<Task>) -> ScoreDirector<Sol, ()> {
    let mut solution = Sol { tasks, score: None };
    solution.update_all_shadows();
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Task",
        "tasks",
        get_tasks,
        get_tasks_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("Sol", TypeId::of::<Sol>()).with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.tasks.len())
}

#[derive(Clone, Copy, Debug)]
struct TestSelector {
    build: fn(&Sol) -> Vec<ScalarMoveUnion<Sol, i32>>,
}

impl MoveSelector<Sol, ScalarMoveUnion<Sol, i32>> for TestSelector {
    type Cursor<'a>
        = ArenaMoveCursor<Sol, ScalarMoveUnion<Sol, i32>>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<Sol>>(
        &'a self,
        score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves((self.build)(score_director.working_solution()))
    }

    fn size<D: solverforge_scoring::Director<Sol>>(&self, score_director: &D) -> usize {
        (self.build)(score_director.working_solution()).len()
    }
}

fn wrap_scalar_composite(
    mov: crate::heuristic::r#move::SequentialCompositeMove<Sol, ScalarMoveUnion<Sol, i32>>,
) -> ScalarMoveUnion<Sol, i32> {
    ScalarMoveUnion::Composite(mov)
}

fn collect_indices<C>(cursor: &mut C) -> Vec<CandidateId>
where
    C: MoveCursor<Sol, ScalarMoveUnion<Sol, i32>>,
{
    collect_cursor_indices::<Sol, ScalarMoveUnion<Sol, i32>, _>(cursor)
}

fn x_move(value: i32) -> ScalarMoveUnion<Sol, i32> {
    ScalarMoveUnion::Change(ChangeMove::new(0, Some(value), get_x, set_x, 0, "x", 0))
}

fn y_move(value: i32) -> ScalarMoveUnion<Sol, i32> {
    ScalarMoveUnion::Change(ChangeMove::new(0, Some(value), get_y, set_y, 0, "y", 0))
}

fn left_x_to_one_then_two(_solution: &Sol) -> Vec<ScalarMoveUnion<Sol, i32>> {
    vec![x_move(1), x_move(2)]
}

fn right_x_to_one(_solution: &Sol) -> Vec<ScalarMoveUnion<Sol, i32>> {
    vec![x_move(1)]
}

fn right_y_to_ten(_solution: &Sol) -> Vec<ScalarMoveUnion<Sol, i32>> {
    vec![y_move(10)]
}

fn right_shadow_backed_y(solution: &Sol) -> Vec<ScalarMoveUnion<Sol, i32>> {
    solution.tasks[0]
        .shadow_y_target
        .map(y_move)
        .into_iter()
        .collect()
}

fn left_increment_x(solution: &Sol) -> Vec<ScalarMoveUnion<Sol, i32>> {
    solution.tasks[0]
        .x
        .map(|value| x_move(value + 1))
        .into_iter()
        .collect()
}

#[test]
fn cartesian_product_arena_yields_all_pairs() {
    let director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);

    let x_selector = ChangeMoveSelector::simple(get_x, set_x, 0, 0, "x", vec![1, 2]);
    let y_selector = ChangeMoveSelector::simple(get_y, set_y, 0, 0, "y", vec![10, 20, 30]);

    let mut arena: CartesianProductArena<Sol, ChangeMove<Sol, i32>, ChangeMove<Sol, i32>> =
        CartesianProductArena::new();

    arena.populate_first(&x_selector, &director);
    arena.populate_second(&y_selector, &director);

    assert_eq!(arena.len(), 6);
    let pairs: Vec<_> = arena.iter_pairs().collect();
    assert_eq!(pairs.len(), 6);
}

#[test]
fn reset_clears_both_arenas() {
    let director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);

    let x_selector = ChangeMoveSelector::simple(get_x, set_x, 0, 0, "x", vec![1, 2]);
    let y_selector = ChangeMoveSelector::simple(get_y, set_y, 0, 0, "y", vec![10, 20]);

    let mut arena: CartesianProductArena<Sol, ChangeMove<Sol, i32>, ChangeMove<Sol, i32>> =
        CartesianProductArena::new();

    arena.populate_first(&x_selector, &director);
    arena.populate_second(&y_selector, &director);
    assert_eq!(arena.len(), 4);

    arena.reset();
    assert!(arena.is_empty());
    assert_eq!(arena.len(), 0);
}

#[test]
fn cartesian_product_skips_rows_with_illegal_left_moves() {
    let mut director = create_director(vec![Task {
        x: Some(1),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let selector = CartesianProductSelector::new(
        TestSelector {
            build: left_x_to_one_then_two,
        },
        TestSelector {
            build: right_y_to_ten,
        },
        wrap_scalar_composite,
    );

    let mut cursor = selector.open_cursor(&director);
    let indices = collect_indices(&mut cursor);

    assert_eq!(selector.size(&director), 2);
    assert_eq!(indices.len(), 1);
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));

    cursor
        .candidate(indices[0])
        .expect("cartesian candidate must remain valid")
        .do_move(&mut director);
    assert_eq!(get_x(director.working_solution(), 0, 0), Some(2));
    assert_eq!(get_y(director.working_solution(), 0, 0), Some(10));
}

#[test]
fn cartesian_product_skips_pairs_with_illegal_right_moves() {
    let mut director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let selector = CartesianProductSelector::new(
        TestSelector {
            build: left_x_to_one_then_two,
        },
        TestSelector {
            build: right_x_to_one,
        },
        wrap_scalar_composite,
    );

    let mut cursor = selector.open_cursor(&director);
    let indices = collect_indices(&mut cursor);

    assert_eq!(selector.size(&director), 2);
    assert_eq!(indices.len(), 1);
    assert!(matches!(
        cursor.candidate(indices[0]),
        Some(MoveCandidateRef::Sequential(_))
    ));
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));

    cursor
        .candidate(indices[0])
        .expect("cartesian candidate must remain valid")
        .do_move(&mut director);
    assert_eq!(get_x(director.working_solution(), 0, 0), Some(1));
}

#[test]
fn cartesian_product_preview_updates_shadows_before_building_right_row() {
    let mut director = create_director(vec![Task {
        x: Some(1),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let selector = CartesianProductSelector::new(
        TestSelector {
            build: |_: &Sol| vec![x_move(2)],
        },
        TestSelector {
            build: right_shadow_backed_y,
        },
        wrap_scalar_composite,
    );

    let mut cursor = selector.open_cursor(&director);
    let indices = collect_indices(&mut cursor);

    assert_eq!(selector.size(&director), 1);
    assert_eq!(indices.len(), 1);
    assert!(indices.iter().all(|&index| cursor
        .candidate(index)
        .is_some_and(|mov| mov.is_doable(&director))));

    let _ = director.calculate_score();
    cursor
        .candidate(indices[0])
        .expect("cartesian candidate must remain valid")
        .do_move(&mut director);
    assert_eq!(get_x(director.working_solution(), 0, 0), Some(2));
    assert_eq!(get_y(director.working_solution(), 0, 0), Some(12));
    assert_eq!(
        director.working_solution().tasks[0].shadow_y_target,
        Some(12)
    );
}

#[test]
fn cartesian_product_moves_remain_stable_after_selector_reuse() {
    let mut director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let selector = CartesianProductSelector::new(
        TestSelector {
            build: left_increment_x,
        },
        TestSelector {
            build: right_shadow_backed_y,
        },
        wrap_scalar_composite,
    );

    let mut first_cursor = selector.open_cursor(&director);
    let first_indices = collect_indices(&mut first_cursor);
    assert_eq!(first_indices.len(), 1);

    set_x(director.working_solution_mut(), 0, 0, Some(5));
    director.working_solution_mut().update_entity_shadows(0, 0);

    let mut second_cursor = selector.open_cursor(&director);
    let second_indices = collect_indices(&mut second_cursor);
    assert_eq!(second_indices.len(), 1);

    first_cursor
        .candidate(first_indices[0])
        .expect("first cartesian candidate must remain valid")
        .do_move(&mut director);
    assert_eq!(get_x(director.working_solution(), 0, 0), Some(1));
    assert_eq!(get_y(director.working_solution(), 0, 0), Some(11));
}

#[derive(Debug)]
struct CountingSelector {
    open_count: std::cell::Cell<usize>,
}

impl CountingSelector {
    fn new() -> Self {
        Self {
            open_count: std::cell::Cell::new(0),
        }
    }
}

impl MoveSelector<Sol, ScalarMoveUnion<Sol, i32>> for CountingSelector {
    type Cursor<'a>
        = ArenaMoveCursor<Sol, ScalarMoveUnion<Sol, i32>>
    where
        Self: 'a;

    fn open_cursor<'a, D: solverforge_scoring::Director<Sol>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        self.open_count.set(self.open_count.get() + 1);
        ArenaMoveCursor::from_moves(vec![x_move(1), x_move(2)])
    }

    fn size<D: solverforge_scoring::Director<Sol>>(&self, _score_director: &D) -> usize {
        2
    }
}

#[test]
fn cartesian_product_size_does_not_open_child_cursors() {
    let director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let left = CountingSelector::new();
    let right = CountingSelector::new();
    let selector = CartesianProductSelector::new(left, right, wrap_scalar_composite);

    assert_eq!(selector.size(&director), 4);
    assert_eq!(selector.left.open_count.get(), 0);
    assert_eq!(selector.right.open_count.get(), 0);
}

#[test]
fn cartesian_product_reverse_signature_matches_true_inverse_order() {
    let mut director = create_director(vec![Task {
        x: Some(0),
        y: Some(0),
        shadow_y_target: None,
    }]);
    let forward_selector = CartesianProductSelector::new(
        TestSelector {
            build: |_| vec![x_move(1)],
        },
        TestSelector {
            build: |_| vec![y_move(10)],
        },
        wrap_scalar_composite,
    );

    let mut forward_cursor = forward_selector.open_cursor(&director);
    let forward_index = collect_indices(&mut forward_cursor)[0];
    let forward_move = forward_cursor.take_candidate(forward_index);
    let forward_signature = forward_move.tabu_signature(&director);
    forward_move.do_move(&mut director);

    let reverse_selector = CartesianProductSelector::new(
        TestSelector {
            build: |_| vec![y_move(0)],
        },
        TestSelector {
            build: |_| vec![x_move(0)],
        },
        wrap_scalar_composite,
    );

    let mut reverse_cursor = reverse_selector.open_cursor(&director);
    let reverse_index = collect_indices(&mut reverse_cursor)[0];
    let reverse_signature = reverse_cursor
        .candidate(reverse_index)
        .expect("reverse cartesian candidate must remain valid")
        .tabu_signature(&director);

    assert_eq!(forward_signature.undo_move_id, reverse_signature.move_id);

    let mut acceptor = TabuSearchAcceptor::<Sol>::new(
        TabuSearchPolicy {
            entity_tabu_size: None,
            value_tabu_size: None,
            move_tabu_size: None,
            undo_move_tabu_size: Some(2),
            aspiration_enabled: false,
        }
        .validated(),
    );
    acceptor.phase_started(&SoftScore::of(0));
    acceptor.step_ended(&SoftScore::of(0), Some(&forward_signature));

    assert!(!acceptor.is_accepted(
        &SoftScore::of(0),
        &SoftScore::of(0),
        Some(&reverse_signature),
    ));
}
