//! Group index and joint placement coverage.

use super::*;

fn placements(member_count: usize, list_len: usize, gap: Option<usize>) -> Vec<Vec<usize>> {
    let mut all = Vec::new();
    let mut placements = JointPlacements::new(member_count, list_len, gap);
    while let Some(positions) = placements.next() {
        all.push(positions.to_vec());
    }
    all
}

#[test]
fn unbounded_pairs_cover_every_increasing_position_pair() {
    let all = placements(2, 3, None);
    assert_eq!(all.len(), 10);
    assert_eq!(all.first(), Some(&vec![0, 1]));
    assert_eq!(all.last(), Some(&vec![3, 4]));
    assert!(all.iter().all(|pair| pair[0] < pair[1] && pair[1] <= 4));
}

#[test]
fn member_gap_bounds_the_elements_between_members() {
    assert_eq!(
        placements(2, 3, Some(0)),
        vec![vec![0, 1], vec![1, 2], vec![2, 3], vec![3, 4]]
    );
    assert_eq!(placements(2, 3, Some(1)).len(), 7);
    let triples = placements(3, 2, Some(1));
    assert!(triples
        .iter()
        .all(|triple| triple[1] - triple[0] <= 2 && triple[2] - triple[1] <= 2));
    assert_eq!(triples.len(), 8);
}

#[test]
fn a_single_member_tries_every_position() {
    assert_eq!(placements(1, 2, Some(0)), vec![vec![0], vec![1], vec![2]]);
}

fn route_score(entity: usize, positions: &[usize]) -> i64 {
    // Owner 2 is best, and members far apart score better.
    let spread = positions.last().unwrap() - positions[0];
    (entity as i64) * 100 + spread as i64 - positions[0] as i64
}

#[test]
fn search_finds_the_best_placement_and_the_runner_up() {
    let JointSearch::Done { best, second } = search_joint_placement(
        &[0, 1, 2],
        2,
        |_| 3,
        JointBounds::default(),
        |entity, positions| Some(route_score(entity, positions)),
    ) else {
        panic!("search was not interrupted");
    };
    let best = best.expect("a placement exists");
    assert_eq!((best.entity, best.positions.to_vec()), (2, vec![0, 4]));
    assert_eq!(best.score, 204);
    assert_eq!(second, Some(203));
}

#[test]
fn the_owner_shortlist_searches_only_the_best_contiguous_owners() {
    let mut evaluated = Vec::new();
    let bounds = JointBounds::from_config(None, std::num::NonZeroU8::new(1));
    let JointSearch::Done { best, .. } = search_joint_placement(
        &[0, 1, 2],
        2,
        |_| 3,
        bounds,
        |entity, positions| {
            evaluated.push((entity, positions.to_vec()));
            Some(route_score(entity, positions))
        },
    ) else {
        panic!("search was not interrupted");
    };
    assert_eq!(best.expect("a placement exists").entity, 2);
    // Four contiguous placements per owner, then owner 2's six others.
    assert_eq!(evaluated.len(), 3 * 4 + 6);
    assert!(evaluated[12..].iter().all(|(entity, _)| *entity == 2));
}

#[test]
fn search_stops_when_eval_does() {
    let mut calls = 0;
    let outcome = search_joint_placement(
        &[0],
        2,
        |_| 3,
        JointBounds::default(),
        |_, _| {
            calls += 1;
            (calls < 3).then_some(0)
        },
    );
    assert_eq!(outcome, JointSearch::<i64>::Interrupted);
    assert_eq!(calls, 3);
}

fn pairs(element: &usize, members: &mut Vec<usize>) {
    // Elements 0..4 pair up as (0, 1) and (2, 3); 4 is alone.
    if *element < 4 {
        let first = element - element % 2;
        members.extend([first, first + 1]);
    }
}

#[test]
fn the_index_keeps_multi_member_groups_in_declared_order() {
    let groups = ListElementGroups::from_hook(5, Some, |element| *element, pairs);
    assert_eq!(groups.group_count(), 2);
    assert_eq!(groups.members(0), &[0, 1]);
    assert_eq!(groups.members(1), &[2, 3]);
    assert_eq!(groups.group_of_key(3), Some(1));
    assert_eq!(groups.group_of_key(4), None);

    let routes = [vec![0, 4, 1], vec![2], vec![]];
    let placement = ListGroupPlacement::locate(
        &groups,
        routes.len(),
        |entity| routes[entity].len(),
        |entity, position| routes[entity].get(position).copied(),
    );
    assert_eq!(placement.member_position(&groups, 0, 1), Some((0, 2)));
    assert!(placement.group_is_assigned(&groups, 0));
    assert!(!placement.group_is_assigned(&groups, 1));
}

#[test]
#[should_panic(expected = "another member declared")]
fn members_that_disagree_about_their_group_are_rejected() {
    ListElementGroups::from_hook(
        3,
        Some,
        |element| *element,
        |element: &usize, members: &mut Vec<usize>| match element {
            0 => members.extend([0, 1]),
            1 => members.extend([1, 2]),
            _ => {}
        },
    );
}
