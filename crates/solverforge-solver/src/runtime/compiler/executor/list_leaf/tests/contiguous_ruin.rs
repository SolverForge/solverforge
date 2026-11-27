//! Contiguous list ruin: adjacent segments from the seed's list and from the
//! list holding the position nearest to the seed.

use std::num::NonZeroU8;

use solverforge_config::{ListRuinMoveSelectorConfig, ListRuinSourceStrategy, MoveSelectorConfig};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCursor, MoveStreamContext};

use super::super::super::super::graph::ListLeafKind;
use super::super::spec::RuntimeListRecipe;
use super::super::{RuntimeListNeighborhoodPlan, RuntimeListNeighborhoodSelector};
use super::support::{director, initial_plan, native_slot, ListPlan, PositionMetric, Slot};

type Selector = RuntimeListNeighborhoodSelector<ListPlan, usize, PositionMetric, PositionMetric>;

fn contiguous(ruin_count: usize, max_source_routes: Option<u8>, slot: Slot) -> Selector {
    let config = MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig {
        min_ruin_count: ruin_count,
        max_ruin_count: ruin_count,
        moves_per_step: Some(40),
        source_strategy: ListRuinSourceStrategy::Contiguous,
        max_source_routes: max_source_routes.and_then(NonZeroU8::new),
        ..ListRuinMoveSelectorConfig::default()
    });
    RuntimeListNeighborhoodSelector::new(
        RuntimeListNeighborhoodPlan::from_compiled(
            ListLeafKind::Ruin,
            &config,
            vec![slot],
            Some(11),
        )
        .expect("contiguous list ruin compiles"),
    )
}

/// Applies and undoes every candidate the selector emits, checking each
/// against `check(sources, routes_after_recreate)`.
fn exercise(
    selector: &Selector,
    mut check: impl FnMut(&[(usize, Vec<usize>)], &[Vec<usize>]),
) -> usize {
    let mut score_director = director(initial_plan());
    let initial = score_director.working_solution().routes.clone();
    let initial_score = score_director.calculate_score();
    let mut stream_state = selector.new_stream_state();
    let mut cursor = selector.open_cursor_with_stream_state(
        &mut stream_state,
        &score_director,
        MoveStreamContext::default(),
    );
    let mut count = 0;
    while let Some(candidate) = cursor.next_owned_candidate(score_director.working_solution()) {
        let RuntimeListRecipe::Ruin { sources, .. } = candidate.clone().into_recipe() else {
            panic!("an ungrouped slot ruins through plain ruin recipes");
        };
        let sources: Vec<(usize, Vec<usize>)> = sources
            .iter()
            .map(|(entity, indices)| (*entity, indices.to_vec()))
            .collect();
        for (_, indices) in &sources {
            assert!(
                indices.windows(2).all(|pair| pair[1] == pair[0] + 1),
                "segment {indices:?} is not contiguous"
            );
        }
        assert!(candidate.is_doable(&score_director));
        let undo = candidate.do_move(&mut score_director);
        let mut elements: Vec<usize> = score_director
            .working_solution()
            .routes
            .iter()
            .flatten()
            .copied()
            .collect();
        elements.sort_unstable();
        assert_eq!(elements, (0..8).collect::<Vec<_>>(), "every stop once");
        check(&sources, &score_director.working_solution().routes);
        candidate.undo_move(&mut score_director, undo);
        assert_eq!(score_director.working_solution().routes, initial);
        assert_eq!(score_director.calculate_score(), initial_score);
        count += 1;
    }
    count
}

#[test]
fn removes_a_segment_on_the_seed_route_and_around_its_nearest_stop() {
    // Routes [0 1 2 3] and [4 5 6 7]; the nearest stop to (route, p) on the
    // other route is position p, so the two segments share a position.
    let count = exercise(
        &contiguous(4, None, Slot::from_static(native_slot(), 0)),
        |sources, _| {
            assert_eq!(sources.len(), 2, "{sources:?}");
            assert_ne!(sources[0].0, sources[1].0);
            assert_eq!(
                sources
                    .iter()
                    .map(|(_, indices)| indices.len())
                    .sum::<usize>(),
                4
            );
            assert!(
                sources[0]
                    .1
                    .iter()
                    .any(|index| sources[1].1.contains(index)),
                "{sources:?} does not start at the seed's nearest stop"
            );
        },
    );
    assert_eq!(count, 40);
}

#[test]
fn one_source_route_keeps_the_segment_on_the_seed_route() {
    let count = exercise(
        &contiguous(3, Some(1), Slot::from_static(native_slot(), 0)),
        |sources, _| {
            assert_eq!(sources.len(), 1, "{sources:?}");
            assert_eq!(sources[0].1.len(), 3);
        },
    );
    assert_eq!(count, 40);
}

/// Stop 2 has no legal owner; odd stops stay on their current route.
fn restricted_owner(_: &ListPlan, element: &usize) -> Option<usize> {
    match element {
        2 => Some(usize::MAX),
        odd if odd % 2 == 1 => Some(odd / 4),
        _ => None,
    }
}

#[test]
fn segments_stop_at_ineligible_stops_and_recreate_keeps_owner_restrictions() {
    let slot = Slot::from_static(
        native_slot().with_element_owner_fn(Some(restricted_owner)),
        0,
    );
    let initial = initial_plan().routes;
    let count = exercise(&contiguous(4, None, slot), |sources, routes| {
        for (entity, indices) in sources {
            assert!(
                indices.iter().all(|&index| initial[*entity][index] != 2),
                "{sources:?} removed or crossed the ineligible stop"
            );
        }
        for (route, stops) in routes.iter().enumerate() {
            for &stop in stops {
                if stop % 2 == 1 {
                    assert_eq!(route, stop / 4, "stop {stop} left its fixed route");
                }
            }
        }
    });
    assert_eq!(count, 40);
}

/// Stops 0 and 1 form a group that leaves and returns together.
fn first_pair(_: &ListPlan, element: usize, members: &mut Vec<usize>) {
    if element < 2 {
        members.extend([0, 1]);
    }
}

#[test]
fn grouped_slots_ruin_whole_groups_across_both_segments() {
    let selector = contiguous(
        3,
        None,
        Slot::from_static(native_slot().with_element_group_fn(Some(first_pair)), 0),
    );
    let mut score_director = director(initial_plan());
    let initial = score_director.working_solution().routes.clone();
    let mut stream_state = selector.new_stream_state();
    let mut cursor = selector.open_cursor_with_stream_state(
        &mut stream_state,
        &score_director,
        MoveStreamContext::default(),
    );
    let mut pair_ruins = 0;
    while let Some(candidate) = cursor.next_owned_candidate(score_director.working_solution()) {
        let RuntimeListRecipe::GroupRuin { plan, .. } = candidate.clone().into_recipe() else {
            panic!("a grouped slot ruins through grouped plans");
        };
        let ruined: Vec<usize> = plan
            .removed
            .iter()
            .map(|&(entity, position)| initial[entity][position])
            .collect();
        assert!(ruined.iter().any(|&stop| stop < 4) && ruined.iter().any(|&stop| stop >= 4));
        if ruined.contains(&0) || ruined.contains(&1) {
            pair_ruins += 1;
            assert!(ruined.contains(&0) && ruined.contains(&1), "{ruined:?}");
        }
        assert!(candidate.is_doable(&score_director));
        let undo = candidate.do_move(&mut score_director);
        candidate.undo_move(&mut score_director, undo);
        assert_eq!(score_director.working_solution().routes, initial);
    }
    assert!(pair_ruins > 0, "some ruin drew the pair");
}
