//! Clone-versus-transfer contract for the shared ruin primitive.
//!
//! Both the public static move and the runtime carrier transfer removed
//! values by ownership: no `CloneProbe::clone` may run during ruin do/undo.

use crate::heuristic::r#move::list_kernel::StaticListRuinAccess;
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::list_kernel::{ruin_do_move, single_ruin_source};
use crate::heuristic::r#move::{ListRuinMove, Move};

#[derive(Debug)]
struct CloneProbe {
    id: usize,
    clone_count: Arc<AtomicUsize>,
}

impl Clone for CloneProbe {
    fn clone(&self) -> Self {
        self.clone_count.fetch_add(1, Ordering::SeqCst);
        Self {
            id: self.id,
            clone_count: Arc::clone(&self.clone_count),
        }
    }
}

impl PartialEq for CloneProbe {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, Debug)]
struct CloneRoute {
    stops: Vec<CloneProbe>,
}

#[derive(Clone, Debug)]
struct ClonePlan {
    routes: Vec<CloneRoute>,
    score: Option<SoftScore>,
}

impl PlanningSolution for ClonePlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn clone_entity_count(solution: &ClonePlan) -> usize {
    solution.routes.len()
}

fn clone_list_len(solution: &ClonePlan, entity: usize) -> usize {
    solution
        .routes
        .get(entity)
        .map_or(0, |route| route.stops.len())
}

fn clone_list_get(solution: &ClonePlan, entity: usize, position: usize) -> Option<CloneProbe> {
    solution
        .routes
        .get(entity)
        .and_then(|route| route.stops.get(position))
        .cloned()
}

fn clone_list_remove(solution: &mut ClonePlan, entity: usize, position: usize) -> CloneProbe {
    solution.routes[entity].stops.remove(position)
}

fn clone_list_insert(solution: &mut ClonePlan, entity: usize, position: usize, value: CloneProbe) {
    solution.routes[entity].stops.insert(position, value);
}

fn director(counter: Arc<AtomicUsize>, stops: Vec<Vec<usize>>) -> ScoreDirector<ClonePlan, ()> {
    let solution = ClonePlan {
        routes: stops
            .into_iter()
            .map(|ids| {
                let counter = Arc::clone(&counter);
                CloneRoute {
                    stops: ids
                        .into_iter()
                        .map(|id| CloneProbe {
                            id,
                            clone_count: Arc::clone(&counter),
                        })
                        .collect(),
                }
            })
            .collect(),
        score: None,
    };
    ScoreDirector::simple(
        solution,
        SolutionDescriptor::new("ClonePlan", TypeId::of::<ClonePlan>()),
        |solution, _| solution.routes.len(),
    )
}

fn static_access() -> StaticListRuinAccess<ClonePlan, CloneProbe> {
    StaticListRuinAccess {
        entity_count: clone_entity_count,
        list_len: clone_list_len,
        list_get: clone_list_get,
        list_remove: clone_list_remove,
        list_insert: clone_list_insert,
        element_owner_fn: None,
        precedence_element_count: None,
        precedence_index_to_element: None,
        precedence_successors_fn: None,
        variable_name: "stops",
        descriptor_index: 0,
    }
}

#[test]
fn ruin_do_move_transfers_removed_values_without_cloning_them() {
    let counter = Arc::new(AtomicUsize::new(0));
    let read_counter = Arc::clone(&counter);
    let mut score_director = director(counter, vec![vec![0, 1, 2], vec![3, 4], vec![5, 6, 7]]);
    read_counter.store(0, Ordering::SeqCst);
    let access = static_access();
    let undo = ruin_do_move(
        &access,
        &single_ruin_source(0, &[0, 1, 2]),
        false,
        &mut score_director,
    );
    assert_eq!(undo.len(), 3);
    assert_eq!(
        read_counter.load(Ordering::SeqCst),
        0,
        "no route element may be cloned while the ruin recirculates owned values"
    );
}

#[test]
fn static_list_ruin_do_and_undo_transfer_values_exactly() {
    let counter = Arc::new(AtomicUsize::new(0));
    let read_counter = Arc::clone(&counter);
    let mut score_director = director(counter, vec![vec![0, 1, 2], vec![3, 4], vec![5, 6, 7]]);
    read_counter.store(0, Ordering::SeqCst);
    let move_ = ListRuinMove::new(
        0,
        &[0, 1, 2],
        clone_entity_count,
        clone_list_len,
        clone_list_get,
        clone_list_remove,
        clone_list_insert,
        "stops",
        0,
    );
    assert!(move_.is_doable(&score_director));
    let undo = move_.do_move(&mut score_director);
    assert_eq!(
        read_counter.load(Ordering::SeqCst),
        0,
        "the static ruin carrier transfers by ownership too"
    );
    move_.undo_move(&mut score_director, undo);
    assert_eq!(
        read_counter.load(Ordering::SeqCst),
        0,
        "undo reinserts removed values without cloning them"
    );
}
