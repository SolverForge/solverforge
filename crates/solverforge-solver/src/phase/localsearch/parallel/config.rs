use solverforge_config::MoveThreadCount;

/* Number of evaluators (main director included) for acceptor-forager local
search. `Auto` stays sequential until benchmarks set its policy; an explicit
count is capped by the threads of the pool the solve runs on.
*/
pub(crate) fn resolve_evaluator_count(count: &MoveThreadCount) -> usize {
    match count {
        MoveThreadCount::Auto | MoveThreadCount::None => 1,
        MoveThreadCount::Count(count) => (*count).clamp(1, rayon::current_num_threads().max(1)),
    }
}
