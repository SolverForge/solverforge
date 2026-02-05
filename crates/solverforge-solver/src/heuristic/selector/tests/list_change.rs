//! Tests for list change move selector.

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::list_change::ListChangeMoveSelector;
use crate::heuristic::selector::MoveSelector;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Vehicle {
    visits: Vec<i32>,
}

#[derive(Clone, Debug)]
struct Solution {
    vehicles: Vec<Vehicle>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for Solution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_vehicles(s: &Solution) -> &Vec<Vehicle> {
    &s.vehicles
}
fn get_vehicles_mut(s: &mut Solution) -> &mut Vec<Vehicle> {
    &mut s.vehicles
}

fn list_len(s: &Solution, entity_idx: usize) -> usize {
    s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
}
fn list_remove(s: &mut Solution, entity_idx: usize, pos: usize) -> Option<i32> {
    s.vehicles.get_mut(entity_idx).map(|v| v.visits.remove(pos))
}
fn list_insert(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
    if let Some(v) = s.vehicles.get_mut(entity_idx) {
        v.visits.insert(pos, val);
    }
}

fn create_director(
    vehicles: Vec<Vehicle>,
) -> SimpleScoreDirector<Solution, impl Fn(&Solution) -> SimpleScore> {
    let solution = Solution {
        vehicles,
        score: None,
    };
    let extractor = Box::new(TypedEntityExtractor::new(
        "Vehicle",
        "vehicles",
        get_vehicles,
        get_vehicles_mut,
    ));
    let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
        .with_extractor(extractor);
    let descriptor =
        SolutionDescriptor::new("Solution", TypeId::of::<Solution>()).with_entity(entity_desc);
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn generates_intra_entity_moves() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3],
    }];
    let director = create_director(vehicles);

    let selector = ListChangeMoveSelector::<Solution, i32, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    // 3 elements. For each position, moves are generated to all other positions
    // EXCEPT forward by 1 (which is a no-op due to index adjustment).
    // From 0: skip 1 (forward by 1), to 2 → 1 move
    // From 1: to 0, skip 2 (forward by 1) → 1 move
    // From 2: to 0, to 1 → 2 moves
    // Total: 4 moves
    assert_eq!(moves.len(), 4);

    // All should be intra-list
    for m in &moves {
        assert!(m.is_intra_list());
    }
}

#[test]
fn generates_inter_entity_moves() {
    let vehicles = vec![Vehicle { visits: vec![1, 2] }, Vehicle { visits: vec![10] }];
    let director = create_director(vehicles);

    let selector = ListChangeMoveSelector::<Solution, i32, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    // Count inter-entity moves
    let inter_count = moves.iter().filter(|m| !m.is_intra_list()).count();
    // Vehicle 0 has 2 elements, each can go to vehicle 1 at positions 0,1 = 4 moves
    // Vehicle 1 has 1 element, can go to vehicle 0 at positions 0,1,2 = 3 moves
    assert_eq!(inter_count, 7);
}

#[test]
fn moves_are_doable() {
    let vehicles = vec![
        Vehicle {
            visits: vec![1, 2, 3],
        },
        Vehicle { visits: vec![10] },
    ];
    let director = create_director(vehicles);

    let selector = ListChangeMoveSelector::<Solution, i32, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    for m in selector.iter_moves(&director) {
        assert!(m.is_doable(&director), "Move should be doable: {:?}", m);
    }
}
