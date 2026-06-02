use std::any::TypeId;
use std::collections::HashSet;

use crate::heuristic::r#move::{ListMoveUnion, Move};
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::list_precedence::ListPrecedenceMoveSelector;
use crate::heuristic::selector::move_selector::{MoveCursor, MoveStreamContext};
use crate::heuristic::selector::MoveSelector;
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

#[derive(Clone, Debug)]
struct Task {
    duration: usize,
    next: Option<usize>,
}

#[derive(Clone, Debug)]
struct Machine {
    tasks: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Plan {
    tasks: Vec<Task>,
    machines: Vec<Machine>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_machines(plan: &Plan) -> &Vec<Machine> {
    &plan.machines
}

fn get_machines_mut(plan: &mut Plan) -> &mut Vec<Machine> {
    &mut plan.machines
}

fn descriptor() -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Machine",
        "machines",
        get_machines,
        get_machines_mut,
    ));
    let entity_desc = EntityDescriptor::new("Machine", TypeId::of::<Machine>(), "machines")
        .with_extractor(extractor);
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(entity_desc)
}

fn create_director(tasks: Vec<Task>, routes: Vec<Vec<usize>>) -> ScoreDirector<Plan, ()> {
    ScoreDirector::simple(
        Plan {
            tasks,
            machines: routes.into_iter().map(|tasks| Machine { tasks }).collect(),
            score: None,
        },
        descriptor(),
        |plan, _| plan.machines.len(),
    )
}

fn element_count(plan: &Plan) -> usize {
    plan.tasks.len()
}

fn index_to_element(_: &Plan, index: usize) -> usize {
    index
}

fn duration(plan: &Plan, task: usize) -> usize {
    plan.tasks[task].duration
}

fn successors(plan: &Plan, task: usize, out: &mut Vec<usize>) {
    if let Some(next) = plan.tasks[task].next {
        out.push(next);
    }
}

fn entity_count(plan: &Plan) -> usize {
    plan.machines.len()
}

fn list_len(plan: &Plan, entity: usize) -> usize {
    plan.machines[entity].tasks.len()
}

fn list_get(plan: &Plan, entity: usize, pos: usize) -> Option<usize> {
    plan.machines[entity].tasks.get(pos).copied()
}

fn list_remove(plan: &mut Plan, entity: usize, pos: usize) -> Option<usize> {
    (pos < plan.machines[entity].tasks.len()).then(|| plan.machines[entity].tasks.remove(pos))
}

fn ruin_remove(plan: &mut Plan, entity: usize, pos: usize) -> usize {
    plan.machines[entity].tasks.remove(pos)
}

fn list_insert(plan: &mut Plan, entity: usize, pos: usize, value: usize) {
    plan.machines[entity].tasks.insert(pos, value);
}

fn list_set(plan: &mut Plan, entity: usize, pos: usize, value: usize) {
    plan.machines[entity].tasks[pos] = value;
}

fn list_reverse(plan: &mut Plan, entity: usize, start: usize, end: usize) {
    plan.machines[entity].tasks[start..end].reverse();
}

fn sublist_remove(plan: &mut Plan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.machines[entity].tasks.drain(start..end).collect()
}

fn sublist_insert(plan: &mut Plan, entity: usize, pos: usize, values: Vec<usize>) {
    plan.machines[entity].tasks.splice(pos..pos, values);
}

fn selector() -> ListPrecedenceMoveSelector<Plan, usize, FromSolutionEntitySelector> {
    ListPrecedenceMoveSelector::new(
        FromSolutionEntitySelector::new(0),
        element_count,
        index_to_element,
        duration,
        successors,
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        list_set,
        list_reverse,
        ruin_remove,
        list_insert,
        None,
        sublist_remove,
        sublist_insert,
        "tasks",
        0,
    )
}

fn move_debug_order(moves: Vec<ListMoveUnion<Plan, usize>>) -> Vec<String> {
    moves.into_iter().map(|mov| format!("{mov:?}")).collect()
}

#[test]
fn list_precedence_selector_emits_critical_block_moves() {
    let director = create_director(
        vec![
            Task {
                duration: 3,
                next: None,
            },
            Task {
                duration: 4,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
        ],
        vec![vec![0, 1, 2]],
    );
    let selector = selector();

    assert_eq!(selector.size(&director), 24);
    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(moves.len(), 24);
    assert!(
        matches!(&moves[0], ListMoveUnion::ListChange(mov) if mov.source_position() == 0 && mov.dest_position() == 2)
    );
    assert!(
        matches!(&moves[1], ListMoveUnion::ListChange(mov) if mov.source_position() == 1 && mov.dest_position() == 3)
    );
    assert!(
        matches!(&moves[2], ListMoveUnion::ListChange(mov) if mov.source_position() == 0 && mov.dest_position() == 3)
    );
    assert!(
        matches!(&moves[3], ListMoveUnion::ListChange(mov) if mov.source_position() == 2 && mov.dest_position() == 0)
    );
    assert!(
        matches!(&moves[4], ListMoveUnion::ListChange(mov) if mov.source_position() == 2 && mov.dest_position() == 1)
    );
    assert!(
        matches!(&moves[5], ListMoveUnion::ListChange(mov) if mov.source_position() == 1 && mov.dest_position() == 0)
    );
    assert!(
        matches!(&moves[6], ListMoveUnion::ListSwap(mov) if mov.first_position() == 0 && mov.second_position() == 1)
    );
    assert!(
        matches!(&moves[7], ListMoveUnion::ListSwap(mov) if mov.first_position() == 0 && mov.second_position() == 2)
    );
    assert!(
        matches!(&moves[8], ListMoveUnion::ListSwap(mov) if mov.first_position() == 1 && mov.second_position() == 2)
    );
    assert!(
        matches!(&moves[9], ListMoveUnion::ListReverse(mov) if mov.entity_index() == 0 && mov.start() == 0 && mov.end() == 2)
    );
    assert!(
        matches!(&moves[10], ListMoveUnion::ListReverse(mov) if mov.entity_index() == 0 && mov.start() == 0 && mov.end() == 3)
    );
    assert!(
        matches!(&moves[11], ListMoveUnion::ListReverse(mov) if mov.entity_index() == 0 && mov.start() == 1 && mov.end() == 3)
    );
    assert!(
        matches!(&moves[12], ListMoveUnion::SublistSwap(mov) if mov.first_entity_index() == 0 && mov.first_start() == 0 && mov.first_end() == 1 && mov.second_start() == 1 && mov.second_end() == 3)
    );
    assert!(
        matches!(&moves[13], ListMoveUnion::SublistSwap(mov) if mov.first_entity_index() == 0 && mov.first_start() == 0 && mov.first_end() == 2 && mov.second_start() == 2 && mov.second_end() == 3)
    );
    assert!(
        matches!(&moves[14], ListMoveUnion::ListRuin(mov) if mov.entity_index() == 0 && mov.element_indices() == [0, 1, 2])
    );
    assert!(
        matches!(&moves[15], ListMoveUnion::SublistChange(mov) if mov.source_start() == 0 && mov.source_end() == 2 && mov.dest_position() == 1)
    );
    assert!(
        matches!(&moves[16], ListMoveUnion::SublistChange(mov) if mov.source_start() == 1 && mov.source_end() == 3 && mov.dest_position() == 0)
    );
    assert!(
        matches!(&moves[17], ListMoveUnion::ListPermute(mov) if mov.start() == 0 && mov.end() == 2 && mov.permutation() == [1, 0])
    );
    assert!(
        matches!(&moves[18], ListMoveUnion::ListPermute(mov) if mov.start() == 0 && mov.end() == 3 && mov.permutation() == [0, 2, 1])
    );
}

#[test]
fn list_precedence_size_matches_streamed_unique_candidates() {
    let director = create_director(
        vec![
            Task {
                duration: 3,
                next: None,
            },
            Task {
                duration: 4,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 2,
                next: Some(2),
            },
        ],
        vec![vec![0, 1, 2], vec![3]],
    );
    let selector = selector();

    let moves = move_debug_order(selector.iter_moves(&director).collect());
    let signatures: HashSet<_> = moves.iter().cloned().collect();

    assert_eq!(moves.len(), selector.size(&director));
    assert_eq!(signatures.len(), moves.len());
}

#[test]
fn list_precedence_selector_streams_all_critical_ruin_windows() {
    let tasks = (0..7)
        .map(|idx| Task {
            duration: idx + 1,
            next: None,
        })
        .collect();
    let director = create_director(tasks, vec![vec![0, 1, 2, 3, 4, 5, 6]]);
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    let ruin_windows = moves
        .iter()
        .filter_map(|mov| match mov {
            ListMoveUnion::ListRuin(mov) => Some(mov.element_indices().to_vec()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(moves.len(), selector.size(&director));
    assert_eq!(
        ruin_windows,
        vec![
            vec![0, 1, 2, 3, 4],
            vec![1, 2, 3, 4, 5],
            vec![2, 3, 4, 5, 6]
        ]
    );
}

#[test]
fn list_precedence_selector_streams_multi_block_ruin_candidates() {
    let tasks = (0..6)
        .map(|_| Task {
            duration: 5,
            next: None,
        })
        .collect();
    let director = create_director(tasks, vec![vec![0, 1, 2], vec![3, 4, 5]]);
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    let multi_ruin = moves
        .iter()
        .filter_map(|mov| match mov {
            ListMoveUnion::ListRuin(mov) if mov.entity_indices().len() == 2 => Some(mov),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(moves.len(), selector.size(&director));
    assert_eq!(multi_ruin.len(), 9);
    assert!(multi_ruin.iter().all(|mov| mov.ruin_count() == 2));
    assert_eq!(multi_ruin[0].entity_indices(), &[0, 1]);
}

#[test]
fn list_precedence_selector_streams_fixed_successor_support_multi_swaps() {
    let director = create_director(
        vec![
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 5,
                next: Some(4),
            },
            Task {
                duration: 10,
                next: None,
            },
            Task {
                duration: 5,
                next: Some(5),
            },
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
        ],
        vec![vec![0, 1], vec![2, 3], vec![4, 5]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    let multi_swaps = moves
        .iter()
        .filter_map(|mov| match mov {
            ListMoveUnion::ListMultiSwap(mov) => Some(mov),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(moves.len(), selector.size(&director));
    assert!(multi_swaps.iter().any(|mov| {
        mov.requires_score_improvement() && mov.swaps() == [(0, 0, 1), (1, 0, 1), (2, 0, 1)]
    }));
}

#[test]
fn list_precedence_selector_applies_stream_context_order() {
    let director = create_director(
        vec![
            Task {
                duration: 3,
                next: None,
            },
            Task {
                duration: 4,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 6,
                next: None,
            },
        ],
        vec![vec![0, 1, 2, 3]],
    );
    let selector = selector();

    let canonical = move_debug_order(selector.iter_moves(&director).collect());
    let mut cursor =
        selector.open_cursor_with_context(&director, MoveStreamContext::new(11, 23, Some(8)));
    let mut contextual = Vec::new();
    while let Some(id) = cursor.next_candidate() {
        contextual.push(format!("{:?}", cursor.take_candidate(id)));
    }

    assert_eq!(canonical.len(), contextual.len());
    let mut canonical_sorted = canonical.clone();
    let mut contextual_sorted = contextual.clone();
    canonical_sorted.sort();
    contextual_sorted.sort();
    assert_eq!(canonical_sorted, contextual_sorted);
    assert_ne!(canonical, contextual);
}

#[test]
fn list_precedence_stream_context_preserves_adjacent_priority_tier_for_short_blocks() {
    let director = create_director(
        vec![
            Task {
                duration: 3,
                next: None,
            },
            Task {
                duration: 4,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 6,
                next: None,
            },
        ],
        vec![vec![0, 1, 2, 3]],
    );
    let selector = selector();

    let mut cursor =
        selector.open_cursor_with_context(&director, MoveStreamContext::new(11, 23, Some(8)));
    let mut first_tier = Vec::new();
    for _ in 0..3 {
        let id = cursor.next_candidate().expect("adjacent critical move");
        first_tier.push(cursor.take_candidate(id));
    }

    assert!(first_tier.iter().all(|mov| {
        matches!(
            mov,
            ListMoveUnion::ListChange(mov)
                if mov.source_entity_index() == 0
                    && mov.dest_entity_index() == 0
                    && mov.dest_position() == mov.source_position() + 2
        )
    }));
}

#[test]
fn list_precedence_selector_ignores_noncritical_route_arcs() {
    let director = create_director(
        vec![
            Task {
                duration: 3,
                next: Some(1),
            },
            Task {
                duration: 2,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
        ],
        vec![vec![0, 2], vec![1]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(moves.len(), 6);
    assert!(
        matches!(&moves[0], ListMoveUnion::ListChange(mov) if mov.source_entity_index() == 0 && mov.source_position() == 0 && mov.dest_position() == 2)
    );
    assert!(
        matches!(&moves[1], ListMoveUnion::ListChange(mov) if mov.source_entity_index() == 0 && mov.source_position() == 1 && mov.dest_position() == 0)
    );
    assert!(
        matches!(&moves[2], ListMoveUnion::ListSwap(mov) if mov.first_entity_index() == 0 && mov.first_position() == 0 && mov.second_position() == 1)
    );
    assert!(
        matches!(&moves[3], ListMoveUnion::ListReverse(mov) if mov.entity_index() == 0 && mov.start() == 0 && mov.end() == 2)
    );
    assert!(
        matches!(&moves[4], ListMoveUnion::ListRuin(mov) if mov.entity_index() == 0 && mov.element_indices() == [0, 1])
    );
    assert!(
        matches!(&moves[5], ListMoveUnion::ListPermute(mov) if mov.entity_index() == 0 && mov.start() == 0 && mov.end() == 2 && mov.permutation() == [1, 0])
    );
}

#[test]
fn list_precedence_selector_skips_moves_that_force_fixed_successor_cycles() {
    let director = create_director(
        vec![
            Task {
                duration: 1,
                next: Some(1),
            },
            Task {
                duration: 1,
                next: None,
            },
        ],
        vec![vec![0, 1]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(selector.size(&director), 1);
    assert_eq!(moves.len(), 1);
    assert!(
        matches!(&moves[0], ListMoveUnion::ListRuin(mov) if mov.entity_index() == 0 && mov.element_indices() == [0, 1])
    );
}

#[test]
fn list_precedence_sublist_destinations_use_route_coordinates() {
    let director = create_director(
        vec![
            Task {
                duration: 1,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
            Task {
                duration: 10,
                next: Some(1),
            },
        ],
        vec![vec![0, 1, 2], vec![3]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(moves.len(), selector.size(&director));
    assert!(moves.iter().any(
        |mov| matches!(mov, ListMoveUnion::SublistChange(mov) if mov.source_start() == 1 && mov.source_end() == 3 && mov.dest_position() == 0)
    ));
    assert!(moves.iter().all(|mov| match mov {
        ListMoveUnion::SublistChange(mov) => mov.source_start() != mov.dest_position(),
        _ => true,
    }));
}

#[test]
fn list_precedence_selector_emits_singleton_critical_node_relocations() {
    let director = create_director(
        vec![
            Task {
                duration: 3,
                next: Some(2),
            },
            Task {
                duration: 1,
                next: None,
            },
            Task {
                duration: 5,
                next: None,
            },
        ],
        vec![vec![0, 1], vec![2]],
    );
    let selector = selector();

    let moves = selector.iter_moves(&director).collect::<Vec<_>>();
    assert_eq!(moves.len(), selector.size(&director));
    assert!(moves.iter().any(
        |mov| matches!(mov, ListMoveUnion::ListChange(mov) if mov.source_entity_index() == 0 && mov.source_position() == 0 && mov.dest_position() == 2)
    ));
}

#[test]
fn list_precedence_selector_skips_cyclic_current_graph() {
    let director = create_director(
        vec![
            Task {
                duration: 1,
                next: Some(1),
            },
            Task {
                duration: 1,
                next: None,
            },
        ],
        vec![vec![1, 0]],
    );
    let selector = selector();

    assert_eq!(selector.size(&director), 0);
    assert_eq!(selector.iter_moves(&director).count(), 0);
}
