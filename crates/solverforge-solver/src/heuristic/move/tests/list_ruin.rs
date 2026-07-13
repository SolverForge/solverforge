// Tests for ListRuinMove operations.

use super::*;
use crate::heuristic::r#move::list_ruin::final_positions_after_insertions;
use smallvec::SmallVec;
use solverforge_core::ConstraintRef;
use solverforge_scoring::{IncrementalConstraint, IncrementalConstraintSealed};

#[derive(Clone, Debug)]
struct Route {
    stops: Vec<i32>,
}

#[derive(Clone, Debug)]
struct VrpSolution {
    routes: Vec<Route>,
    score: Option<SoftScore>,
}

impl PlanningSolution for VrpSolution {
    type Score = SoftScore;
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
fn list_get(s: &VrpSolution, entity_idx: usize, pos: usize) -> Option<i32> {
    s.routes
        .get(entity_idx)
        .and_then(|r| r.stops.get(pos))
        .copied()
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

fn two_node_cycle_element_count(_: &VrpSolution) -> usize {
    2
}

fn two_node_cycle_index_to_element(_: &VrpSolution, idx: usize) -> i32 {
    i32::try_from(idx + 1).expect("test element index fits i32")
}

fn two_node_cycle_successors(_: &VrpSolution, element: i32, out: &mut Vec<i32>) {
    match element {
        1 => out.push(2),
        2 => out.push(1),
        _ => {}
    }
}

fn create_director(stops: Vec<i32>) -> ScoreDirector<VrpSolution, ()> {
    let routes = vec![Route { stops }];
    let solution = VrpSolution {
        routes,
        score: None,
    };
    ScoreDirector::simple(solution, solution_descriptor(), |s, _| s.routes.len())
}

struct RouteScoreConstraint {
    constraint_ref: ConstraintRef,
    score_fn: fn(&VrpSolution) -> SoftScore,
    current_score: SoftScore,
}

impl RouteScoreConstraint {
    fn new(score_fn: fn(&VrpSolution) -> SoftScore) -> Self {
        Self {
            constraint_ref: ConstraintRef::new("", "routeScore"),
            score_fn,
            current_score: SoftScore::ZERO,
        }
    }
}

impl IncrementalConstraintSealed for RouteScoreConstraint {}

impl IncrementalConstraint<VrpSolution, SoftScore> for RouteScoreConstraint {
    fn evaluate(&self, solution: &VrpSolution) -> SoftScore {
        (self.score_fn)(solution)
    }

    fn match_count(&self, _solution: &VrpSolution) -> usize {
        1
    }

    fn initialize(&mut self, solution: &VrpSolution) -> SoftScore {
        self.current_score = self.evaluate(solution);
        self.current_score
    }

    fn on_insert(
        &mut self,
        solution: &VrpSolution,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> SoftScore {
        let next_score = self.evaluate(solution);
        let delta = next_score - self.current_score;
        self.current_score = next_score;
        delta
    }

    fn on_retract(
        &mut self,
        _solution: &VrpSolution,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> SoftScore {
        SoftScore::ZERO
    }

    fn reset(&mut self) {
        self.current_score = SoftScore::ZERO;
    }

    fn constraint_ref(&self) -> &ConstraintRef {
        &self.constraint_ref
    }
}

fn create_director_with_score(
    stops: Vec<i32>,
    score_fn: fn(&VrpSolution) -> SoftScore,
) -> ScoreDirector<VrpSolution, RouteScoreConstraint> {
    let solution = VrpSolution {
        routes: vec![Route { stops }],
        score: None,
    };
    ScoreDirector::with_descriptor(
        solution,
        RouteScoreConstraint::new(score_fn),
        solution_descriptor(),
        |s, _| s.routes.len(),
    )
}

fn prefer_four_before_two(solution: &VrpSolution) -> SoftScore {
    let Some(route) = solution.routes.first() else {
        return SoftScore::ZERO;
    };
    let pos_four = route
        .stops
        .iter()
        .position(|&stop| stop == 4)
        .unwrap_or(usize::MAX);
    let pos_two = route
        .stops
        .iter()
        .position(|&stop| stop == 2)
        .unwrap_or(usize::MAX);
    SoftScore::of(if pos_four < pos_two { 100 } else { 0 })
}

fn solution_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Route",
        "routes",
        get_routes,
        get_routes_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Route", TypeId::of::<Route>(), "routes").with_extractor(extractor);
    SolutionDescriptor::new("VrpSolution", TypeId::of::<VrpSolution>()).with_entity(entity_desc)
}

#[test]
fn ruin_single_element() {
    let mut director = create_director(vec![1, 2, 3, 4, 5]);

    let m = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[2],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(m.is_doable(&director));
    assert_eq!(m.ruin_count(), 1);

    {
        let mut recording = SnapshotDirector::new(&mut director);
        m.do_move(&mut recording);

        /* After ruin-and-recreate: element 3 (was at index 2) is reinserted.
        With constant score=0, first tried position wins: (entity=0, pos=0).
        Route contains same elements, just possibly reordered.
        */
        let stops = &director.working_solution().routes[0].stops;
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
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(m.is_doable(&director));
    assert_eq!(m.ruin_count(), 2);

    {
        let mut recording = SnapshotDirector::new(&mut director);
        m.do_move(&mut recording);

        // After ruin-and-recreate: elements 2 and 4 removed then reinserted.
        // Same elements, possibly reordered.
        let stops = &director.working_solution().routes[0].stops;
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
fn ruin_recreate_can_choose_removed_elements_out_of_removal_order() {
    let mut director = create_director_with_score(vec![1, 2, 3, 4], prefer_four_before_two);

    let m = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[1, 3],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    let undo = m.do_move(&mut director);
    let stops = &director.working_solution().routes[0].stops;
    let pos_four = stops.iter().position(|&stop| stop == 4).unwrap();
    let pos_two = stops.iter().position(|&stop| stop == 2).unwrap();

    assert!(
        pos_four < pos_two,
        "recreate should choose element 4 before element 2 when that scores better"
    );

    m.undo_move(&mut director, undo);

    assert_eq!(
        director.working_solution().routes[0].stops,
        vec![1, 2, 3, 4]
    );
}

#[test]
fn ruin_recreate_restores_multiple_source_entities() {
    let mut director = create_director_with_score(vec![1, 2, 3, 4], prefer_four_before_two);
    director.working_solution_mut().routes.push(Route {
        stops: vec![5, 6, 7],
    });

    let m = ListRuinMove::<VrpSolution, i32>::new_multi_source(
        &[
            (0, SmallVec::from_slice(&[1, 3])),
            (1, SmallVec::from_slice(&[0, 2])),
        ],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    let undo = m.do_move(&mut director);
    assert_eq!(m.ruin_count(), 4);
    assert_eq!(m.entity_indices(), &[0, 1]);

    m.undo_move(&mut director, undo);

    assert_eq!(
        director.working_solution().routes[0].stops,
        vec![1, 2, 3, 4]
    );
    assert_eq!(director.working_solution().routes[1].stops, vec![5, 6, 7]);
}

#[test]
fn ruin_unordered_indices() {
    let mut director = create_director(vec![1, 2, 3, 4, 5]);

    let m = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[3, 1],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    {
        let mut recording = SnapshotDirector::new(&mut director);
        m.do_move(&mut recording);

        // Indices sorted internally: removes index 1 and 3 (values 2 and 4).
        let stops = &director.working_solution().routes[0].stops;
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
        list_get,
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
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    );

    assert!(!m.is_doable(&director));
}

#[test]
fn computes_exact_final_positions_for_same_entity_reinsertion() {
    let placements = SmallVec::<[(usize, usize); 8]>::from_slice(&[(0, 0), (0, 0), (0, 0), (0, 1)]);

    let current = final_positions_after_insertions(&placements);

    assert_eq!(current.as_slice(), &[3, 2, 0, 1]);
}

#[test]
fn undo_positions_do_not_underflow_for_interacting_same_entity_insertions() {
    let placements = SmallVec::<[(usize, usize); 8]>::from_slice(&[(0, 0), (0, 0), (0, 0), (0, 1)]);
    let mut current = final_positions_after_insertions(&placements);
    let mut removal_order = Vec::new();

    for i in (0..placements.len()).rev() {
        let (entity_i, _) = placements[i];
        let actual_pos = current[i];
        removal_order.push(actual_pos);

        for j in 0..i {
            let (entity_j, _) = placements[j];
            if entity_j == entity_i && current[j] > actual_pos {
                current[j] -= 1;
            }
        }
    }

    assert_eq!(removal_order, vec![1, 0, 0, 0]);
}

#[test]
fn precedence_ruin_restores_original_when_recreate_has_no_safe_position() {
    let mut director = create_director(vec![1, 2]);

    let mov = ListRuinMove::<VrpSolution, i32>::new(
        0,
        &[0],
        entity_count,
        list_len,
        list_get,
        list_remove,
        list_insert,
        "stops",
        0,
    )
    .with_precedence_hooks(
        Some(two_node_cycle_element_count),
        Some(two_node_cycle_index_to_element),
        Some(two_node_cycle_successors),
    );

    let undo = mov.do_move(&mut director);

    assert!(undo.is_empty());
    assert_eq!(director.working_solution().routes[0].stops, vec![1, 2]);
}

mod transfer;
