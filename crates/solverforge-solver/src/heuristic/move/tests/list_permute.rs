use smallvec::smallvec;

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
        .and_then(|v| v.visits.get(pos))
        .copied()
}

fn sublist_remove(
    s: &mut RoutingSolution,
    entity_idx: usize,
    start: usize,
    end: usize,
) -> Vec<i32> {
    s.vehicles
        .get_mut(entity_idx)
        .map(|v| v.visits.drain(start..end).collect())
        .unwrap_or_default()
}

fn sublist_insert(s: &mut RoutingSolution, entity_idx: usize, pos: usize, items: Vec<i32>) {
    if let Some(v) = s.vehicles.get_mut(entity_idx) {
        for (i, item) in items.into_iter().enumerate() {
            v.visits.insert(pos + i, item);
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
fn permutes_window_and_undo_restores_original_order() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3, 4, 5],
    }];
    let mut director = create_director(vehicles);
    let mov = ListPermuteMove::<RoutingSolution, i32>::new(
        0,
        1,
        4,
        smallvec![2, 0, 1],
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    assert!(mov.is_doable(&director));

    {
        let mut recording = SnapshotDirector::new(&mut director);
        mov.do_move(&mut recording);
        assert_eq!(
            director.working_solution().vehicles[0].visits,
            &[1, 4, 2, 3, 5]
        );
        recording.undo_changes();
    }

    assert_eq!(
        director.working_solution().vehicles[0].visits,
        &[1, 2, 3, 4, 5]
    );
}

#[test]
fn rejects_identity_and_duplicate_permutations() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3],
    }];
    let director = create_director(vehicles);
    let identity = ListPermuteMove::<RoutingSolution, i32>::new(
        0,
        0,
        3,
        smallvec![0, 1, 2],
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );
    let duplicate = ListPermuteMove::<RoutingSolution, i32>::new(
        0,
        0,
        3,
        smallvec![0, 0, 2],
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    assert!(!identity.is_doable(&director));
    assert!(!duplicate.is_doable(&director));
}
