//! One partitioned-search pass: split the working solution, solve each
//! partition in its own child scope, merge. Shared by
//! [`super::PartitionedSearchPhase`] and the configured runtime's
//! `partitioned_search` phase.

use rand::RngExt;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::manager::SolverTerminalReason;
use crate::scope::ProgressCallback;
use crate::scope::{PartitionBound, PendingControl, SolverScope, SolverScopeChildConfig};
use solverforge_config::TerminationConfig;

use super::config::PartitionedSearchConfig;
use super::partitioner::SolutionPartitioner;

enum PartitionOutcome<S> {
    Complete(S),
    Pause,
    Cancelled,
    Terminated,
    /// A queued problem change stopped the child. The parent returns without
    /// merging so the runner applies the change to the unmerged solution.
    ProblemChange,
}

/// Partitions the working solution with `partitioner`, solves every
/// partition with `solve_children` on up to `config.thread_count` threads,
/// and merges the solved partitions into the working solution.
///
/// `make_director` builds each partition's director from the parent's on the
/// calling thread;
/// the partition's first score calculation, on the thread that solves it,
/// initializes it. A pause re-partitions from the parent's working solution
/// once resumed; cancellation, termination and a queued problem change stop
/// the pass without merging.
///
/// `termination` is the phase's own: its time limit, counted from the
/// phase's start, bounds every partition, and its step count limit bounds
/// each partition's solve. A partition that reaches one ends like a phase:
/// its best solution is merged and the solve goes on.
pub(crate) fn solve_partitioned<S, D, BestCb, PD, Part, MakeDirector, SolveChildren>(
    solver_scope: &mut SolverScope<'_, S, D, BestCb>,
    partitioner: &Part,
    config: &PartitionedSearchConfig,
    termination: Option<&TerminationConfig>,
    mut make_director: MakeDirector,
    solve_children: &SolveChildren,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    PD: Director<S>,
    Part: SolutionPartitioner<S>,
    MakeDirector: FnMut(&D, S) -> PD,
    SolveChildren: Fn(&mut SolverScope<'_, S, PD>) + Sync,
{
    let bound = PartitionBound::new(solver_scope.partition_bound(), termination);
    'partitioning: loop {
        if solver_scope.should_terminate() || bound.as_ref().is_some_and(PartitionBound::reached) {
            return;
        }

        let solution = solver_scope.score_director().working_solution().clone();
        let partitions = partitioner.partition(&solution);
        let partition_count = partitions.len();
        if partition_count == 0 {
            return;
        }

        let thread_count = config
            .thread_count
            .resolve(partition_count, partitioner.recommended_partition_count());
        if config.log_progress {
            tracing::info!(
                event = "phase_start",
                phase = "PartitionedSearch",
                partition_count,
                thread_count,
            );
        }

        let directors: Vec<PD> = partitions
            .into_iter()
            .map(|partition| make_director(solver_scope.score_director(), partition))
            .collect();
        let partition_bounds: Vec<PartitionBound<'_>> = bound
            .iter()
            .flat_map(|bound| directors.iter().map(|_| bound.for_partition()))
            .collect();
        let children: Vec<(PD, u64, Option<&PartitionBound<'_>>)> = directors
            .into_iter()
            .enumerate()
            .map(|(index, director)| {
                (
                    director,
                    solver_scope.rng().random(),
                    partition_bounds.get(index),
                )
            })
            .collect();
        let phase_budget = solver_scope.child_phase_budget();
        let child_config = solver_scope.child_config(Some(&phase_budget));
        let outcomes = solve_partitions(children, thread_count, child_config, solve_children);

        let mut solved_partitions = Vec::with_capacity(outcomes.len());
        for outcome in outcomes {
            match outcome {
                PartitionOutcome::Complete(partition) => solved_partitions.push(partition),
                PartitionOutcome::Pause => {
                    solver_scope.pause_if_requested();
                    continue 'partitioning;
                }
                PartitionOutcome::Cancelled => {
                    solver_scope.mark_cancelled();
                    return;
                }
                PartitionOutcome::Terminated => {
                    solver_scope.mark_terminated_by_config();
                    return;
                }
                PartitionOutcome::ProblemChange => return,
            }
        }

        // A phase end — the enclosing phase's termination or partition
        // bound — still merges what the partitions solved; only a stop of
        // the solve itself abandons them.
        if solver_scope.should_terminate()
            && (solver_scope.yielded_to_parent()
                || solver_scope.terminal_reason() != SolverTerminalReason::Completed)
        {
            return;
        }

        let merged = partitioner.merge(&solution, solved_partitions);
        solver_scope.replace_working_solution_and_reinitialize(merged);
        solver_scope.update_best_solution();

        if config.log_progress {
            if let Some(score) = solver_scope.best_score() {
                tracing::info!(
                    event = "phase_end",
                    phase = "PartitionedSearch",
                    score = %format!("{:?}", score),
                );
            }
        }
        return;
    }
}

fn solve_partitions<'t, S, PD, SolveChildren>(
    children: Vec<(PD, u64, Option<&'t PartitionBound<'t>>)>,
    thread_count: usize,
    child_config: SolverScopeChildConfig<'t, S>,
    solve_children: &SolveChildren,
) -> Vec<PartitionOutcome<S>>
where
    S: PlanningSolution,
    PD: Director<S>,
    SolveChildren: Fn(&mut SolverScope<'_, S, PD>) + Sync,
{
    if thread_count <= 1 || children.len() <= 1 {
        return children
            .into_iter()
            .map(|(director, seed, bound)| {
                let child_config = child_config.clone().with_partition_bound(bound);
                solve_partition(director, child_config, seed, solve_children)
            })
            .collect();
    }

    ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .expect("failed to build partitioned search rayon pool")
        .install(|| {
            children
                .into_par_iter()
                .map(|(director, seed, bound)| {
                    let child_config = child_config.clone().with_partition_bound(bound);
                    solve_partition(director, child_config, seed, solve_children)
                })
                .collect()
        })
}

fn solve_partition<'t, S, PD, SolveChildren>(
    director: PD,
    child_config: SolverScopeChildConfig<'t, S>,
    seed: u64,
    solve_children: &SolveChildren,
) -> PartitionOutcome<S>
where
    S: PlanningSolution,
    PD: Director<S>,
    SolveChildren: Fn(&mut SolverScope<'_, S, PD>) + Sync,
{
    let mut solver_scope = child_config.build_scope(director, seed);
    // A partition starting after its phase's time limit is left as it is,
    // like any unreached work at a phase end; a stop of the solve itself is
    // classified below.
    if !solver_scope.should_terminate() {
        solver_scope.initialize_working_solution_as_best();
        solve_children(&mut solver_scope);
    }

    match solver_scope.pending_control() {
        PendingControl::PauseRequested => return PartitionOutcome::Pause,
        PendingControl::CancelRequested => return PartitionOutcome::Cancelled,
        // Only the partition bound: the phase ended, the partition is done.
        PendingControl::ConfigTerminationRequested if !solver_scope.solve_limit_reached() => {}
        PendingControl::ConfigTerminationRequested => return PartitionOutcome::Terminated,
        PendingControl::ProblemChangeRequested => return PartitionOutcome::ProblemChange,
        PendingControl::Continue => {}
    }
    if solver_scope.yielded_to_parent() {
        return PartitionOutcome::Pause;
    }
    match solver_scope.terminal_reason() {
        SolverTerminalReason::Cancelled => return PartitionOutcome::Cancelled,
        SolverTerminalReason::TerminatedByConfig => return PartitionOutcome::Terminated,
        SolverTerminalReason::Completed | SolverTerminalReason::Failed => {}
    }

    PartitionOutcome::Complete(solver_scope.take_best_or_working_solution())
}
