use solverforge_config::MoveThreadCount;

/// Evaluators `move_thread_count = "auto"` uses at most, main director
/// included. Each evaluator beyond the main director holds a replica (a
/// working-solution clone with its own incremental score state), so `auto`
/// bounds the count to bound that memory; an explicit count can go higher.
pub(crate) const AUTO_MAX_EVALUATORS: usize = 4;

/// How acceptor-forager local search evaluates each step's candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationPolicy {
    /// The main director runs every trial.
    Sequential,
    /// Parallel windows across this many evaluators (at least two).
    Fixed(usize),
    /// Per step, sequential or parallel windows across up to this many
    /// evaluators (at least two), chosen from measured costs.
    Auto(usize),
}

/// Resolves `move_thread_count` against the threads of the pool the solve
/// runs on: a pool of one thread is always sequential, and an explicit count
/// is capped at the pool's threads. The evaluator pool caps counts again at
/// one full window of candidates.
pub(crate) fn resolve_evaluation_policy(count: &MoveThreadCount) -> EvaluationPolicy {
    let threads = rayon::current_num_threads().max(1);
    match count {
        MoveThreadCount::None => EvaluationPolicy::Sequential,
        MoveThreadCount::Count(count) => match count.get().min(threads) {
            1 => EvaluationPolicy::Sequential,
            evaluators => EvaluationPolicy::Fixed(evaluators),
        },
        MoveThreadCount::Auto => match threads.min(AUTO_MAX_EVALUATORS) {
            1 => EvaluationPolicy::Sequential,
            evaluators => EvaluationPolicy::Auto(evaluators),
        },
    }
}
