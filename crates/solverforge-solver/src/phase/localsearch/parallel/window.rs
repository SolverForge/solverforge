use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCandidateUndo};
use crate::phase::localsearch::evaluation::{run_trial, TrialOutcome};

use super::pool::{EvaluatorPool, TrialOutputs};

/* Runs the trial for every candidate in the window. The main director takes
its share inline on the coordinator thread; each replica takes its share in a
spawned task. Slots are handed out through one atomic counter, and results land
at their slot index, so which evaluator ran a slot never affects the outcome.
*/
pub(crate) fn evaluate_window<S, D, M>(
    candidates: &[MoveCandidateRef<'_, S, M>],
    main_director: &mut D,
    main_journal_pool: &mut Vec<MoveCandidateUndo<M::Undo>>,
    evaluators: &mut EvaluatorPool<S, D, M>,
    results: &mut Vec<Option<(TrialOutcome<S::Score>, Duration)>>,
) where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    let next = AtomicUsize::new(0);
    let (main_outputs, replicas) = evaluators.parts_mut();
    main_outputs.clear();
    rayon::scope(|scope| {
        for replica in replicas.iter_mut() {
            let next = &next;
            replica.outputs.clear();
            scope.spawn(move |_| {
                drain_window(
                    candidates,
                    next,
                    &mut replica.director,
                    &mut replica.journal_pool,
                    &mut replica.outputs,
                );
            });
        }
        drain_window(
            candidates,
            &next,
            main_director,
            main_journal_pool,
            main_outputs,
        );
    });
    results.clear();
    results.resize_with(candidates.len(), || None);
    for (index, outcome, elapsed) in main_outputs.drain(..) {
        results[index] = Some((outcome, elapsed));
    }
    for replica in replicas.iter_mut() {
        for (index, outcome, elapsed) in replica.outputs.drain(..) {
            results[index] = Some((outcome, elapsed));
        }
    }
}

fn drain_window<S, D, M>(
    candidates: &[MoveCandidateRef<'_, S, M>],
    next: &AtomicUsize,
    director: &mut D,
    journal_pool: &mut Vec<MoveCandidateUndo<M::Undo>>,
    outputs: &mut TrialOutputs<S::Score>,
) where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    loop {
        let index = next.fetch_add(1, Ordering::Relaxed);
        let Some(candidate) = candidates.get(index) else {
            break;
        };
        let started = Instant::now();
        let outcome = run_trial(candidate, director, journal_pool);
        outputs.push((index, outcome, started.elapsed()));
    }
}
