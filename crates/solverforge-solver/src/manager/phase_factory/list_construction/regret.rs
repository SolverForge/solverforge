//! Public facade for canonical source-indexed regret insertion.

#[cfg(test)]
mod facade;
mod kernel;

#[cfg(test)]
pub(crate) use facade::ListRegretInsertionPhase;
pub(crate) use kernel::run_regret;

#[cfg(test)]
#[path = "regret/tests.rs"]
mod tests;
