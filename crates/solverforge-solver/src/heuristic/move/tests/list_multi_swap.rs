// Tests for ListMultiSwapMove operations.

use super::*;

#[derive(Clone, Debug)]
struct Vehicle {
    visits: Vec<i32>,
}

#[derive(Clone, Debug)]
struct RoutingSolution {
    vehicles: Vec<Vehicle>,
    score: Option<SoftScore>,
}

impl PlanningSolution for RoutingSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_vehicles(s: &RoutingSolution) -> &Vec<Vehicle> {
    &s.vehicles
}

fn get_vehicles_mut(s: &mut RoutingSolution) -> &mut Vec<Vehicle> {
    &mut s.vehicles
}

fn list_len(s: &RoutingSolution, entity_idx: usize) -> usize {
    s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
}

fn list_get(s: &RoutingSolution, entity_idx: usize, pos: usize) -> Option<i32> {
    s.vehicles
        .get(entity_idx)
        .and_then(|v| v.visits.get(pos).copied())
}

fn list_set(s: &mut RoutingSolution, entity_idx: usize, pos: usize, val: i32) {
    if let Some(v) = s.vehicles.get_mut(entity_idx) {
        if let Some(elem) = v.visits.get_mut(pos) {
            *elem = val;
        }
    }
}

fn create_director(vehicles: Vec<Vehicle>) -> ScoreDirector<RoutingSolution, ()> {
    let solution = RoutingSolution {
        vehicles,
        score: None,
    };
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Vehicle",
        "vehicles",
        get_vehicles,
        get_vehicles_mut,
    ));
    let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
        .with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("RoutingSolution", TypeId::of::<RoutingSolution>())
        .with_entity(entity_desc);
    ScoreDirector::simple(solution, descriptor, |s, _| s.vehicles.len())
}

#[test]
fn multi_swap_applies_independent_intra_list_swaps_and_undoes() {
    let vehicles = vec![
        Vehicle {
            visits: vec![1, 2, 3],
        },
        Vehicle {
            visits: vec![10, 20, 30],
        },
        Vehicle {
            visits: vec![100, 200, 300],
        },
    ];
    let mut director = create_director(vehicles);

    let m = ListMultiSwapMove::<RoutingSolution, i32>::new(
        &[(0, 0, 2), (1, 0, 1), (2, 1, 2)],
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );

    assert!(m.is_doable(&director));
    assert_eq!(m.entity_indices(), &[0, 1, 2]);

    {
        let mut recording = SnapshotDirector::new(&mut director);
        m.do_move(&mut recording);

        let sol = director.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![3, 2, 1]);
        assert_eq!(sol.vehicles[1].visits, vec![20, 10, 30]);
        assert_eq!(sol.vehicles[2].visits, vec![100, 300, 200]);

        recording.undo_changes();
    }

    let sol = director.working_solution();
    assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3]);
    assert_eq!(sol.vehicles[1].visits, vec![10, 20, 30]);
    assert_eq!(sol.vehicles[2].visits, vec![100, 200, 300]);
}

#[test]
fn multi_swap_rejects_overlapping_entities() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3],
    }];
    let director = create_director(vehicles);

    let m = ListMultiSwapMove::<RoutingSolution, i32>::new(
        &[(0, 0, 1), (0, 1, 2)],
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );

    assert!(!m.is_doable(&director));
}

#[test]
fn multi_swap_tabu_identity_is_swap_order_stable() {
    let vehicles = vec![
        Vehicle {
            visits: vec![1, 2, 3],
        },
        Vehicle {
            visits: vec![10, 20, 30],
        },
        Vehicle {
            visits: vec![100, 200, 300],
        },
    ];
    let director = create_director(vehicles);

    let first = ListMultiSwapMove::<RoutingSolution, i32>::new(
        &[(2, 1, 2), (0, 0, 2), (1, 0, 1)],
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );
    let second = ListMultiSwapMove::<RoutingSolution, i32>::new(
        &[(1, 0, 1), (2, 2, 1), (0, 2, 0)],
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );

    let first_signature = first.tabu_signature(&director);
    let second_signature = second.tabu_signature(&director);

    assert_eq!(first_signature.move_id, first_signature.undo_move_id);
    assert_eq!(first_signature.move_id, second_signature.move_id);
}
