use std::any::TypeId;
use std::collections::HashSet;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::list_permute::ListPermuteMoveSelector;
use crate::heuristic::selector::MoveSelector;
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

#[derive(Clone, Debug)]
struct Vehicle {
    visits: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Plan {
    vehicles: Vec<Vehicle>,
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

fn get_vehicles(plan: &Plan) -> &Vec<Vehicle> {
    &plan.vehicles
}

fn get_vehicles_mut(plan: &mut Plan) -> &mut Vec<Vehicle> {
    &mut plan.vehicles
}

fn descriptor() -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Vehicle",
        "vehicles",
        get_vehicles,
        get_vehicles_mut,
    ));
    let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
        .with_extractor(extractor);
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(entity_desc)
}

fn create_director(vehicles: Vec<Vehicle>) -> ScoreDirector<Plan, ()> {
    ScoreDirector::simple(
        Plan {
            vehicles,
            score: None,
        },
        descriptor(),
        |plan, _| plan.vehicles.len(),
    )
}

fn list_len(plan: &Plan, entity_idx: usize) -> usize {
    plan.vehicles
        .get(entity_idx)
        .map_or(0, |vehicle| vehicle.visits.len())
}

fn list_get(plan: &Plan, entity_idx: usize, pos: usize) -> Option<usize> {
    plan.vehicles
        .get(entity_idx)
        .and_then(|vehicle| vehicle.visits.get(pos))
        .copied()
}

fn sublist_remove(plan: &mut Plan, entity_idx: usize, start: usize, end: usize) -> Vec<usize> {
    plan.vehicles
        .get_mut(entity_idx)
        .map(|vehicle| vehicle.visits.drain(start..end).collect())
        .unwrap_or_default()
}

fn sublist_insert(plan: &mut Plan, entity_idx: usize, pos: usize, items: Vec<usize>) {
    if let Some(vehicle) = plan.vehicles.get_mut(entity_idx) {
        for (offset, item) in items.into_iter().enumerate() {
            vehicle.visits.insert(pos + offset, item);
        }
    }
}

fn fixed_owner(_: &Plan, element: &usize) -> Option<usize> {
    Some(*element / 100)
}

fn selector(
    min_window_size: usize,
    max_window_size: usize,
) -> ListPermuteMoveSelector<Plan, usize, FromSolutionEntitySelector> {
    ListPermuteMoveSelector::new(
        FromSolutionEntitySelector::new(0),
        min_window_size,
        max_window_size,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    )
}

#[test]
fn list_permute_enumerates_windows_without_batching_moves() {
    let director = create_director(vec![Vehicle {
        visits: vec![1, 2, 3],
    }]);
    let selector = selector(2, 3);

    assert_eq!(selector.size(&director), 7);

    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(moves.len(), 7);
    assert!(moves.iter().all(|mov| mov.is_doable(&director)));
    assert_eq!(moves[0].start(), 0);
    assert_eq!(moves[0].end(), 2);
    assert_eq!(moves[0].permutation(), &[1, 0]);
    assert!(moves.iter().all(|mov| mov.entity_index() == 0));
}

#[test]
fn list_permute_size_matches_streamed_unique_candidates() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![1, 2, 3, 4],
        },
        Vehicle {
            visits: vec![100, 101, 102],
        },
    ]);
    let selector = selector(2, 3);

    let moves: Vec<_> = selector.iter_moves(&director).collect();
    let signatures: HashSet<_> = moves
        .iter()
        .map(|mov| {
            (
                mov.entity_index(),
                mov.start(),
                mov.end(),
                mov.permutation().to_vec(),
            )
        })
        .collect();

    assert_eq!(moves.len(), selector.size(&director));
    assert_eq!(signatures.len(), moves.len());
}

#[test]
fn list_permute_respects_fixed_owner_restrictions() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![0, 100, 1],
        },
        Vehicle {
            visits: vec![101, 102],
        },
    ]);
    let selector = selector(2, 2).with_element_owner_fn(Some(fixed_owner));

    let moves: Vec<_> = selector.iter_moves(&director).collect();
    assert_eq!(selector.size(&director), 1);
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].entity_index(), 1);
    assert_eq!(moves[0].start(), 0);
    assert_eq!(moves[0].permutation(), &[1, 0]);
}
