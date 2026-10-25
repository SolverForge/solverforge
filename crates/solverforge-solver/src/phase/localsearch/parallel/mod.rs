/* Parallel local-search evaluation.

The main director and persistent evaluator replicas run trials for one window
of candidates at a time; the coordinator consumes the results in candidate
order through the unchanged acceptor and forager.
*/

mod budget;
mod config;
mod pool;
mod window;

#[cfg(test)]
mod tests;

pub(crate) use budget::remaining_evaluation_budget;
pub(crate) use config::resolve_evaluator_count;
pub(crate) use pool::{EvaluatorPool, MAX_PARALLEL_EVALUATION_WINDOW};
pub(crate) use window::evaluate_window;
