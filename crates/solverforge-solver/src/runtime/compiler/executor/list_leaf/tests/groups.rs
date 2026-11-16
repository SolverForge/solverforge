//! Group change, group swap, grouped ruin and the pinning of grouped
//! elements in the other list families.

use std::num::NonZeroU8;

use solverforge_config::{
    ListGroupChangeMoveConfig, ListRuinMoveSelectorConfig, MoveSelectorConfig, SelectionOrder,
};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCursor, MoveStreamContext};

use super::super::super::super::graph::ListLeafKind;
use super::super::spec::RuntimeListRecipe;
use super::super::{RuntimeListNeighborhoodPlan, RuntimeListNeighborhoodSelector};
use super::support::{director, initial_plan, native_slot, ListPlan, PositionMetric, Slot};

/// Elements below `GROUPED` pair up as (0, 1), (2, 3); the rest are alone.
const GROUPED: usize = 8;

fn pairs(_: &ListPlan, element: usize, members: &mut Vec<usize>) {
    if element < GROUPED {
        let first = element - element % 2;
        members.extend([first, first + 1]);
    }
}

fn first_pair_only(_: &ListPlan, element: usize, members: &mut Vec<usize>) {
    if element < 2 {
        members.extend([0, 1]);
    }
}

fn grouped_slot(hook: fn(&ListPlan, usize, &mut Vec<usize>)) -> Slot {
    Slot::from_static(native_slot().with_element_group_fn(Some(hook)), 0)
}

type Selector = RuntimeListNeighborhoodSelector<ListPlan, usize, PositionMetric, PositionMetric>;

fn selector(kind: ListLeafKind, config: MoveSelectorConfig, slot: Slot) -> Selector {
    RuntimeListNeighborhoodSelector::new(
        RuntimeListNeighborhoodPlan::from_compiled(kind, &config, vec![slot], Some(7))
            .expect("test list plan compiles"),
    )
}

fn group_change(max_member_distance: Option<u8>) -> MoveSelectorConfig {
    MoveSelectorConfig::ListGroupChangeMoveSelector(ListGroupChangeMoveConfig {
        selection_order: Some(SelectionOrder::Original),
        max_member_distance: max_member_distance.and_then(NonZeroU8::new),
        ..ListGroupChangeMoveConfig::default()
    })
}

/// Every pair sits on one route with its first member first.
fn assert_pairs_together(routes: &[Vec<usize>], pair_count: usize) {
    for pair in 0..pair_count {
        let (first, second) = (2 * pair, 2 * pair + 1);
        let route = routes
            .iter()
            .find(|route| route.contains(&first))
            .expect("every element stays assigned");
        let first_position = route.iter().position(|&element| element == first).unwrap();
        let second_position = route.iter().position(|&element| element == second);
        assert!(
            second_position.is_some_and(|second_position| first_position < second_position),
            "pair ({first}, {second}) split or reordered in {routes:?}"
        );
    }
    let mut all: Vec<usize> = routes.iter().flatten().copied().collect();
    all.sort_unstable();
    assert_eq!(
        all,
        (0..8).collect::<Vec<_>>(),
        "every element exactly once"
    );
}

/// Applies and undoes every doable candidate; returns how many there were.
fn exercise(selector: &Selector, pair_count: usize, kind: &str) -> usize {
    let mut score_director = director(initial_plan());
    let initial = score_director.working_solution().routes.clone();
    let mut stream_state = selector.new_stream_state();
    let mut cursor = selector.open_cursor_with_stream_state(
        &mut stream_state,
        &score_director,
        MoveStreamContext::default(),
    );
    let mut moves = Vec::new();
    while let Some(candidate) = cursor.next_owned_candidate(score_director.working_solution()) {
        moves.push(candidate);
    }
    let mut doable = 0;
    for candidate in moves {
        if !candidate.is_doable(&score_director) {
            continue;
        }
        doable += 1;
        let undo = candidate.do_move(&mut score_director);
        assert_pairs_together(&score_director.working_solution().routes, pair_count);
        assert_ne!(
            score_director.working_solution().routes,
            initial,
            "{kind} candidate {candidate:?} changed nothing"
        );
        candidate.undo_move(&mut score_director, undo);
        assert_eq!(
            score_director.working_solution().routes,
            initial,
            "{kind} undo restores the lists"
        );
    }
    doable
}

#[test]
fn group_change_moves_every_pair_jointly_to_every_placement() {
    // Per pair: 5 other placements on its own route, 15 on the other one.
    let all = selector(
        ListLeafKind::GroupChange,
        group_change(None),
        grouped_slot(pairs),
    );
    assert_eq!(exercise(&all, 4, "group change"), 4 * (5 + 15));
    // Adjacent members only: 2 on its own route, 5 on the other one.
    let adjacent = selector(
        ListLeafKind::GroupChange,
        group_change(Some(1)),
        grouped_slot(pairs),
    );
    assert_eq!(exercise(&adjacent, 4, "adjacent group change"), 4 * (2 + 5));
}

#[test]
fn group_swap_exchanges_equal_groups_between_owners() {
    let swap = selector(
        ListLeafKind::GroupSwap,
        MoveSelectorConfig::ListGroupSwapMoveSelector(Default::default()),
        grouped_slot(pairs),
    );
    // Pairs (0, 1) and (2, 3) on route 0 against (4, 5) and (6, 7).
    assert_eq!(exercise(&swap, 4, "group swap"), 4);
}

#[test]
fn grouped_ruin_takes_whole_groups_and_recreates_them_together() {
    let ruin = selector(
        ListLeafKind::Ruin,
        MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig {
            min_ruin_count: 1,
            max_ruin_count: 3,
            moves_per_step: Some(12),
            max_member_distance: NonZeroU8::new(2),
            ..ListRuinMoveSelectorConfig::default()
        }),
        grouped_slot(first_pair_only),
    );
    let mut score_director = director(initial_plan());
    let initial = score_director.working_solution().routes.clone();
    let mut stream_state = ruin.new_stream_state();
    let mut cursor = ruin.open_cursor_with_stream_state(
        &mut stream_state,
        &score_director,
        MoveStreamContext::default(),
    );
    let mut grouped_ruins = 0;
    while let Some(candidate) = cursor.next_owned_candidate(score_director.working_solution()) {
        let RuntimeListRecipe::GroupRuin { plan, .. } = candidate.clone().into_recipe() else {
            panic!("a grouped slot ruins through grouped plans");
        };
        // A ruined member brings its partner along as one unit.
        let ruined: Vec<usize> = plan
            .removed
            .iter()
            .map(|&(entity, position)| initial[entity][position])
            .collect();
        if ruined.contains(&0) || ruined.contains(&1) {
            grouped_ruins += 1;
            assert!(ruined.contains(&0) && ruined.contains(&1), "{ruined:?}");
        }
        assert!(candidate.is_doable(&score_director));
        let undo = candidate.do_move(&mut score_director);
        assert_pairs_together(&score_director.working_solution().routes, 1);
        candidate.undo_move(&mut score_director, undo);
        assert_eq!(score_director.working_solution().routes, initial);
    }
    assert!(grouped_ruins > 0, "some ruin drew the pair");
}

#[test]
fn single_element_moves_keep_grouped_elements_on_their_owner() {
    let change = selector(
        ListLeafKind::Change,
        super::support::config(ListLeafKind::Change),
        grouped_slot(first_pair_only),
    );
    let score_director = director(initial_plan());
    let mut stream_state = change.new_stream_state();
    let mut cursor = change.open_cursor_with_stream_state(
        &mut stream_state,
        &score_director,
        MoveStreamContext::default(),
    );
    let mut crossings = 0;
    while let Some(candidate) = cursor.next_owned_candidate(score_director.working_solution()) {
        let RuntimeListRecipe::Change { coordinates, .. } = candidate.into_recipe() else {
            panic!("list change emits change recipes");
        };
        let element = initial_plan().routes[coordinates.source_entity][coordinates.source_position];
        if coordinates.source_entity != coordinates.destination_entity {
            assert!(element >= 2, "grouped element {element} left its owner");
            crossings += 1;
        }
    }
    assert!(crossings > 0, "ungrouped elements still change owner");
}
