//! Clone-versus-transfer regression coverage for the shared ruin primitive.

use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::list_kernel::{
    ruin_do_move, single_ruin_source, RuinValueTransfer, StaticListRuinAccess,
};
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

fn director(counter: Arc<AtomicUsize>) -> ScoreDirector<ClonePlan, ()> {
    let solution = ClonePlan {
        routes: vec![CloneRoute {
            stops: vec![CloneProbe {
                id: 1,
                clone_count: counter,
            }],
        }],
        score: None,
    };
    ScoreDirector::simple(
        solution,
        SolutionDescriptor::new("ClonePlan", TypeId::of::<ClonePlan>()),
        |solution, _| solution.routes.len(),
    )
}

fn static_clone_count(counter: Arc<AtomicUsize>) -> usize {
    let mut score_director = director(Arc::clone(&counter));
    counter.store(0, Ordering::SeqCst);
    let move_ = ListRuinMove::new(
        0,
        &[0],
        clone_entity_count,
        clone_list_len,
        clone_list_get,
        clone_list_remove,
        clone_list_insert,
        "stops",
        0,
    );
    let _ = move_.do_move(&mut score_director);
    counter.load(Ordering::SeqCst)
}

fn transfer_clone_count(counter: Arc<AtomicUsize>) -> usize {
    let mut score_director = director(Arc::clone(&counter));
    counter.store(0, Ordering::SeqCst);
    let access = StaticListRuinAccess {
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
    };
    let _ = ruin_do_move(
        &access,
        &single_ruin_source(0, &[0]),
        false,
        RuinValueTransfer::MoveIntoInsert,
        &mut score_director,
    );
    counter.load(Ordering::SeqCst)
}

#[test]
fn static_ruin_retains_historic_final_insertion_clone_while_owned_carriers_may_transfer() {
    let static_count = static_clone_count(Arc::new(AtomicUsize::new(0)));
    let transfer_count = transfer_clone_count(Arc::new(AtomicUsize::new(0)));

    assert_eq!(
        static_count,
        transfer_count + 1,
        "static ListRuinMove must retain its final-insertion clone; only an owned runtime carrier may transfer that element"
    );
}
