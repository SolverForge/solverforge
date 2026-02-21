//! Tests for ListRuinMove operations.

use super::*;

#[derive(Clone, Debug)]
struct Route {
    stops: Vec<i32>,
}

#[derive(Clone, Debug)]
struct VrpSolution {
    routes: Vec<Route>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for VrpSolution {
    type Score = SimpleScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_routes(s: &VrpSolution) -> &Vec<Route> {
    &s.routes
}
fn get_routes_mut(s: &mut VrpSolution) -> &mut Vec<Route> {
    &mut s.routes
}

fn entity_count(s: &VrpSolution) -> usize {
    s.routes.len()
}
fn list_len(s: &VrpSolution, entity_idx: usize) -> usize {
    s.routes.get(entity_idx).map_or(0, |r| r.stops.len())
}
fn list_remove(s: &mut VrpSolution, entity_idx: usize, idx: usize) -> i32 {
    s.routes
        .get_mut(entity_idx)
        .map(|r| r.stops.remove(idx))
        .unwrap_or(0)
}
fn list_insert(s: &mut VrpSolution, entity_idx: usize, idx: usize, v: i32) {
    if let Some(r) = s.routes.get_mut(entity_idx) {
        r.stops.insert(idx, v);
    }
}

fn create_director(
    stops: Vec<i32>,
) -> SimpleScoreDirector<VrpSolution, impl Fn(&VrpSolution) -> SimpleScore> {
    let routes = vec![Route { stops }];
    let solution = VrpSolution {
        routes,
        score: None,
    };
    let extractor = Box::new(TypedEntityExtractor::new(
        "Route",
        "routes",
        get_routes,
        get_routes_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Route", TypeId::of::<Route>(), "routes").with_extractor(extractor);
    let descriptor = SolutionDescriptor::new("VrpSolution", TypeId::of::<VrpSolution>())
        .with_entity(entity_desc);
    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn ruin_single_element() {
    let mut director = create_director(vec![1, 2, 3, 4, 5]);

    let m = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[2],
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(m.is_doable(&director));
    assert_eq!(m.ruin_count(), 1);

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        // After ruin-and-recreate: element 3 (was at index 2) is reinserted.
        // With constant score=0, first tried position wins: (entity=0, pos=0).
        // Route contains same elements, just possibly reordered.
        let stops = &recording.working_solution().routes[0].stops;
        assert_eq!(stops.len(), 5);
        let mut sorted = stops.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5]);

        recording.undo_changes();
    }

    // Undo must restore original exactly
    let stops = &director.working_solution().routes[0].stops;
    assert_eq!(stops, &[1, 2, 3, 4, 5]);
}

#[test]
fn ruin_multiple_elements() {
    let mut director = create_director(vec![1, 2, 3, 4, 5]);

    let m = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[1, 3],
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(m.is_doable(&director));
    assert_eq!(m.ruin_count(), 2);

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        // After ruin-and-recreate: elements 2 and 4 removed then reinserted.
        // Same elements, possibly reordered.
        let stops = &recording.working_solution().routes[0].stops;
        assert_eq!(stops.len(), 5);
        let mut sorted = stops.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5]);

        recording.undo_changes();
    }

    let stops = &director.working_solution().routes[0].stops;
    assert_eq!(stops, &[1, 2, 3, 4, 5]);
}

#[test]
fn ruin_unordered_indices() {
    let mut director = create_director(vec![1, 2, 3, 4, 5]);

    let m = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[3, 1],
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    {
        let mut recording = RecordingScoreDirector::new(&mut director);
        m.do_move(&mut recording);

        // Indices sorted internally: removes index 1 and 3 (values 2 and 4).
        let stops = &recording.working_solution().routes[0].stops;
        assert_eq!(stops.len(), 5);
        let mut sorted = stops.clone();
        sorted.sort();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5]);

        recording.undo_changes();
    }

    let stops = &director.working_solution().routes[0].stops;
    assert_eq!(stops, &[1, 2, 3, 4, 5]);
}

#[test]
fn empty_indices_not_doable() {
    let director = create_director(vec![1, 2, 3]);

    let m = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[],
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(!m.is_doable(&director));
}

#[test]
fn out_of_bounds_not_doable() {
    let director = create_director(vec![1, 2, 3]);

    let m = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[0, 10],
        entity_count,
        list_len,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(!m.is_doable(&director));
}
