// Tests for SublistChangeMove operations.

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
fn intra_list_move_forward() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3, 4, 5, 6],
    }];
    let mut director = create_director(vehicles);

    let m = SublistChangeMove::<RoutingSolution, i32>::new(
        0,
        1,
        3,
        0,
        4,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingDirector::new(&mut director);
        m.do_move(&mut recording);

        let visits = &recording.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 4, 5, 6, 2, 3]);

        recording.undo_changes();
    }

    let visits = &director.working_solution().vehicles[0].visits;
    assert_eq!(visits, &[1, 2, 3, 4, 5, 6]);
}

#[test]
fn intra_list_move_backward() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3, 4, 5, 6],
    }];
    let mut director = create_director(vehicles);

    let m = SublistChangeMove::<RoutingSolution, i32>::new(
        0,
        3,
        5,
        0,
        1,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingDirector::new(&mut director);
        m.do_move(&mut recording);

        let visits = &recording.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 4, 5, 2, 3, 6]);

        recording.undo_changes();
    }

    let visits = &director.working_solution().vehicles[0].visits;
    assert_eq!(visits, &[1, 2, 3, 4, 5, 6]);
}

#[test]
fn inter_list_move() {
    let vehicles = vec![
        Vehicle {
            visits: vec![1, 2, 3, 4],
        },
        Vehicle {
            visits: vec![10, 20],
        },
    ];
    let mut director = create_director(vehicles);

    let m = SublistChangeMove::<RoutingSolution, i32>::new(
        0,
        1,
        3,
        1,
        1,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingDirector::new(&mut director);
        m.do_move(&mut recording);

        let sol = recording.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![1, 4]);
        assert_eq!(sol.vehicles[1].visits, vec![10, 2, 3, 20]);

        recording.undo_changes();
    }

    let sol = director.working_solution();
    assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3, 4]);
    assert_eq!(sol.vehicles[1].visits, vec![10, 20]);
}

#[test]
fn empty_range_not_doable() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3],
    }];
    let director = create_director(vehicles);

    let m = SublistChangeMove::<RoutingSolution, i32>::new(
        0,
        2,
        2,
        0,
        0,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    assert!(!m.is_doable(&director));
}

#[test]
fn out_of_bounds_not_doable() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3],
    }];
    let director = create_director(vehicles);

    let m = SublistChangeMove::<RoutingSolution, i32>::new(
        0,
        1,
        10,
        0,
        0,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    assert!(!m.is_doable(&director));
}

#[test]
fn dest_at_source_start_not_doable() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3, 4, 5],
    }];
    let director = create_director(vehicles);

    let m = SublistChangeMove::<RoutingSolution, i32>::new(
        0,
        1,
        4,
        0,
        1,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    assert!(!m.is_doable(&director));
}

#[test]
fn intra_list_rightward_tabu_inverse_matches_reverse_move() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3, 4, 5, 6],
    }];
    let mut director = create_director(vehicles);

    let m = SublistChangeMove::<RoutingSolution, i32>::new(
        0,
        1,
        3,
        0,
        4,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );
    let signature = m.tabu_signature(&director);

    {
        let mut recording = RecordingDirector::new(&mut director);
        m.do_move(&mut recording);

        let reverse = SublistChangeMove::<RoutingSolution, i32>::new(
            0,
            4,
            6,
            0,
            1,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            "visits",
            0,
        );
        assert!(reverse.is_doable(&recording));

        let reverse_signature = reverse.tabu_signature(&recording);
        assert_ne!(signature.move_id, signature.undo_move_id);
        assert_eq!(signature.undo_move_id, reverse_signature.move_id);

        recording.undo_changes();
    }

    assert_eq!(
        director.working_solution().vehicles[0].visits,
        vec![1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn inter_list_tabu_inverse_matches_reverse_move() {
    let vehicles = vec![
        Vehicle {
            visits: vec![1, 2, 3, 4],
        },
        Vehicle {
            visits: vec![10, 20],
        },
    ];
    let mut director = create_director(vehicles);

    let m = SublistChangeMove::<RoutingSolution, i32>::new(
        0,
        1,
        3,
        1,
        1,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );
    let signature = m.tabu_signature(&director);

    {
        let mut recording = RecordingDirector::new(&mut director);
        m.do_move(&mut recording);

        let reverse = SublistChangeMove::<RoutingSolution, i32>::new(
            1,
            1,
            3,
            0,
            1,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            "visits",
            0,
        );
        assert!(reverse.is_doable(&recording));

        let reverse_signature = reverse.tabu_signature(&recording);
        assert_ne!(signature.move_id, signature.undo_move_id);
        assert_eq!(signature.undo_move_id, reverse_signature.move_id);

        recording.undo_changes();
    }

    let solution = director.working_solution();
    assert_eq!(solution.vehicles[0].visits, vec![1, 2, 3, 4]);
    assert_eq!(solution.vehicles[1].visits, vec![10, 20]);
}
