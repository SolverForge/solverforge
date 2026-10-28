// Partitioned search yields to a queued problem change without merging.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_scoring::{Director, ScoreDirector};

use crate::manager::{SolverRuntime, SolverTerminalReason};
use crate::phase::partitioned::{FunctionalPartitioner, PartitionedSearchPhase};
use crate::phase::Phase;
use crate::realtime::ClosureProblemChange;
use crate::scope::{ProgressCallback, SolverScope};
use crate::test_utils::{create_nqueens_descriptor, create_nqueens_director, NQueensSolution};

#[derive(Debug)]
struct QueueChangeThenSearch {
    runtime: SolverRuntime<NQueensSolution>,
    steps: Arc<AtomicUsize>,
}

impl<D, BestCb> Phase<NQueensSolution, D, BestCb> for QueueChangeThenSearch
where
    D: Director<NQueensSolution>,
    BestCb: ProgressCallback<NQueensSolution>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, NQueensSolution, D, BestCb>) {
        self.runtime
            .submit_problem_change_for_test(Box::new(
                ClosureProblemChange::<NQueensSolution, _>::new("arrives mid-child", |_| {}),
            ))
            .expect("solving job accepts a change");
        for _ in 0..1_000 {
            if solver_scope.should_terminate() {
                return;
            }
            self.steps.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "QueueChangeThenSearch"
    }
}

#[test]
fn pending_problem_change_yields_children_without_merging() {
    let runtime = SolverRuntime::detached();
    let mut solver_scope =
        SolverScope::new(create_nqueens_director(&[0, 1, 2, 3])).with_runtime(Some(runtime));
    solver_scope.initialize_working_solution_as_best();

    let merges = Arc::new(AtomicUsize::new(0));
    let merges_seen = Arc::clone(&merges);
    let partitioner = FunctionalPartitioner::new(
        |solution: &NQueensSolution| vec![solution.clone()],
        move |_original, _partitions| {
            merges_seen.fetch_add(1, Ordering::SeqCst);
            NQueensSolution::with_rows(&[3, 3, 3, 3])
        },
    );
    let steps = Arc::new(AtomicUsize::new(0));
    let child_steps = Arc::clone(&steps);
    let mut phase = PartitionedSearchPhase::new(
        partitioner,
        |solution: NQueensSolution| {
            ScoreDirector::simple(solution, create_nqueens_descriptor(), |s, _| s.queens.len())
        },
        move || {
            (QueueChangeThenSearch {
                runtime,
                steps: Arc::clone(&child_steps),
            },)
        },
    );

    phase.solve(&mut solver_scope);

    assert_eq!(
        steps.load(Ordering::SeqCst),
        0,
        "the child stops at its first poll"
    );
    assert_eq!(merges.load(Ordering::SeqCst), 0);
    let rows: Vec<_> = solver_scope
        .working_solution()
        .queens
        .iter()
        .map(|queen| queen.row)
        .collect();
    assert_eq!(rows, vec![Some(0), Some(1), Some(2), Some(3)]);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::Completed
    );
    assert!(
        runtime.is_problem_change_pending(),
        "only the top-level runner drains the queue"
    );
}
