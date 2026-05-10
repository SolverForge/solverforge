// PartitionedSearchPhase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;

use rand::RngExt;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::manager::SolverTerminalReason;
use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::{PendingControl, SolverScope, SolverScopeChildConfig};

use super::child_phases::ChildPhases;
use super::config::PartitionedSearchConfig;
use super::partitioner::SolutionPartitioner;

enum PartitionOutcome<S> {
    Complete(S),
    Pause,
    Cancelled,
    Terminated,
}

/// Partitioned search phase that solves partitions in parallel.
///
/// This phase:
/// 1. Partitions the solution using the provided partitioner
/// 2. Creates a solver for each partition
/// 3. Runs child phases on each partition in parallel
/// 4. Merges the solved partitions back together
///
/// Each partition runs independently with its own solver scope.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `PD` - The score director type for partition solvers
/// * `Part` - The partitioner type (implements `SolutionPartitioner<S>`)
/// * `SDF` - The score director factory function type
/// * `PF` - The phase factory function type
/// * `CP` - The child phases type (tuple of phases)
pub struct PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    PD: Director<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    // The partitioner that splits and merges solutions.
    partitioner: Part,

    // Factory for creating score directors for each partition.
    score_director_factory: SDF,

    // Factory for creating child phases for each partition.
    phase_factory: PF,

    // Configuration for this phase.
    config: PartitionedSearchConfig,

    _marker: PhantomData<(fn() -> S, fn() -> PD, fn() -> CP)>,
}

impl<S, PD, Part, SDF, PF, CP> PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    PD: Director<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    pub fn new(partitioner: Part, score_director_factory: SDF, phase_factory: PF) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config: PartitionedSearchConfig::default(),
            _marker: PhantomData,
        }
    }

    pub fn with_config(
        partitioner: Part,
        score_director_factory: SDF,
        phase_factory: PF,
        config: PartitionedSearchConfig,
    ) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config,
            _marker: PhantomData,
        }
    }
}

impl<S, PD, Part, SDF, PF, CP> Debug for PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    PD: Director<S>,
    Part: SolutionPartitioner<S> + Debug,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionedSearchPhase")
            .field("partitioner", &self.partitioner)
            .field("config", &self.config)
            .finish()
    }
}

impl<S, D, BestCb, PD, Part, SDF, PF, CP> Phase<S, D, BestCb>
    for PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    PD: Director<S> + 'static,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD> + Send,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        'partitioning: loop {
            if solver_scope.should_terminate() {
                return;
            }

            let solution = solver_scope.score_director().working_solution().clone();
            let partitions = self.partitioner.partition(&solution);
            let partition_count = partitions.len();

            if partition_count == 0 {
                return;
            }

            let thread_count = self.config.thread_count.resolve(partition_count);

            if self.config.log_progress {
                tracing::info!(event = "phase_start", phase = "PartitionedSearch",);
            }

            let child_seeds: Vec<u64> = (0..partition_count)
                .map(|_| solver_scope.rng().random())
                .collect();
            let phase_budget = solver_scope.child_phase_budget();
            let child_config = solver_scope.child_config(Some(&phase_budget));
            let outcomes =
                self.solve_partitions(partitions, thread_count, child_config, child_seeds);

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
                }
            }

            if solver_scope.should_terminate() {
                return;
            }

            let merged = self.partitioner.merge(&solution, solved_partitions);
            solver_scope.replace_working_solution_and_reinitialize(merged);
            solver_scope.update_best_solution();

            if self.config.log_progress {
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

    fn phase_type_name(&self) -> &'static str {
        "PartitionedSearch"
    }
}

impl<S, PD, Part, SDF, PF, CP> PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    PD: Director<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    // Solves a single partition and returns the solved solution.
    fn solve_partition<'t>(
        &self,
        partition: S,
        child_config: SolverScopeChildConfig<'t, S>,
        seed: u64,
    ) -> PartitionOutcome<S> {
        // Create score director for this partition
        let director = (self.score_director_factory)(partition);

        // Create solver scope
        let mut solver_scope = child_config.build_scope(director, seed);
        if solver_scope.should_terminate() {
            return PartitionOutcome::Terminated;
        }
        solver_scope.initialize_working_solution_as_best();

        // Create and run child phases
        let mut phases = (self.phase_factory)();
        phases.solve_all(&mut solver_scope);

        match solver_scope.pending_control() {
            PendingControl::PauseRequested => return PartitionOutcome::Pause,
            PendingControl::CancelRequested => return PartitionOutcome::Cancelled,
            PendingControl::ConfigTerminationRequested => return PartitionOutcome::Terminated,
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

    fn solve_partitions<'t>(
        &self,
        partitions: Vec<S>,
        thread_count: usize,
        child_config: SolverScopeChildConfig<'t, S>,
        child_seeds: Vec<u64>,
    ) -> Vec<PartitionOutcome<S>> {
        if thread_count <= 1 || partitions.len() <= 1 {
            return partitions
                .into_iter()
                .zip(child_seeds)
                .map(|(partition, seed)| {
                    self.solve_partition(partition, child_config.clone(), seed)
                })
                .collect();
        }

        ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .expect("failed to build partitioned search rayon pool")
            .install(|| {
                partitions
                    .into_par_iter()
                    .zip(child_seeds.into_par_iter())
                    .map(|(partition, seed)| {
                        self.solve_partition(partition, child_config.clone(), seed)
                    })
                    .collect()
            })
    }
}

#[cfg(test)]
#[path = "phase_tests.rs"]
mod tests;
