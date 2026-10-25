use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rayon::prelude::*;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCandidateRef;
use crate::phase::localsearch::evaluation::{run_trial, TrialOutcome};

use super::pool::{EvaluatorPool, TrialOutputs};

/* Runs the trial for every candidate in the window. One fork-join splits the
work: the main director drains slots on the coordinator thread while the
replicas drain theirs through a parallel iterator. Fork-join jobs live on the
stack, so dispatch allocates nothing. Slots are handed out through one atomic
counter, and results land at their slot index, so which evaluator ran a slot
never affects the outcome.
*/
pub(crate) fn evaluate_window<S, D, M>(
    candidates: &[MoveCandidateRef<'_, S, M>],
    main_director: &mut D,
    main_journal_pool: &mut Vec<M::Undo>,
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
    rayon::join(
        || {
            drain_window(
                candidates,
                &next,
                main_director,
                main_journal_pool,
                main_outputs,
            )
        },
        || {
            replicas.par_iter_mut().for_each(|replica| {
                replica.outputs.clear();
                drain_window(
                    candidates,
                    &next,
                    &mut replica.director,
                    &mut replica.journal_pool,
                    &mut replica.outputs,
                );
            })
        },
    );
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
    journal_pool: &mut Vec<M::Undo>,
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
