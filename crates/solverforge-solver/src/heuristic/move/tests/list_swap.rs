//! Tests for ListSwapMove operations.

use super::*;

#[derive(Clone, Debug)]
struct Vehicle {
    visits: Vec<i32>,
}

#[derive(Clone, Debug)]
struct RoutingSolution {
    vehicles: Vec<Vehicle>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for RoutingSolution {
    type Score = SimpleScore;
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

fn create_director(
    vehicles: Vec<Vehicle>,
) -> SimpleScoreDirector<RoutingSolution, impl Fn(&RoutingSolution) -> SimpleScore> {
    let solution = RoutingSolution {
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
    let descriptor = SolutionDescriptor::new("RoutingSolution", TypeId::of::<RoutingSolution>())
        .with_entity(entity_desc);
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn intra_list_swap() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3, 4, 5],
    }];
    let mut director = create_director(vehicles);

    let m = ListSwapMove::<RoutingSolution, i32>::new(
        0, 1, 0, 3, list_len, list_get, list_set, "visits", 0,
    );

    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        let visits = &recording.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 4, 3, 2, 5]);

        recording.undo_changes();
    }

    let visits = &director.working_solution().vehicles[0].visits;
    assert_eq!(visits, &[1, 2, 3, 4, 5]);
}

#[test]
fn inter_list_swap() {
    let vehicles = vec![
        Vehicle {
            visits: vec![1, 2, 3],
        },
        Vehicle {
            visits: vec![10, 20, 30],
        },
    ];
    let mut director = create_director(vehicles);

    let m = ListSwapMove::<RoutingSolution, i32>::new(
        0, 1, 1, 2, list_len, list_get, list_set, "visits", 0,
    );

    assert!(m.is_doable(&director));

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        let sol = recording.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![1, 30, 3]);
        assert_eq!(sol.vehicles[1].visits, vec![10, 20, 2]);

        recording.undo_changes();
    }

    let sol = director.working_solution();
    assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3]);
    assert_eq!(sol.vehicles[1].visits, vec![10, 20, 30]);
}

#[test]
fn same_position_not_doable() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3],
    }];
    let director = create_director(vehicles);

    let m = ListSwapMove::<RoutingSolution, i32>::new(
        0, 1, 0, 1, list_len, list_get, list_set, "visits", 0,
    );

    assert!(!m.is_doable(&director));
}

#[test]
fn same_values_not_doable() {
    let vehicles = vec![Vehicle {
        visits: vec![5, 5, 5],
    }];
    let director = create_director(vehicles);

    let m = ListSwapMove::<RoutingSolution, i32>::new(
        0, 0, 0, 2, list_len, list_get, list_set, "visits", 0,
    );

    assert!(!m.is_doable(&director));
}

#[test]
fn invalid_position_not_doable() {
    let vehicles = vec![Vehicle {
        visits: vec![1, 2, 3],
    }];
    let director = create_director(vehicles);

    let m = ListSwapMove::<RoutingSolution, i32>::new(
        0, 1, 0, 10, list_len, list_get, list_set, "visits", 0,
    );

    assert!(!m.is_doable(&director));
}
