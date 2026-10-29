//! Public facade for canonical source-indexed cheapest insertion.

#[cfg(test)]
mod facade;
mod kernel;
mod live;

#[cfg(test)]
pub(crate) use facade::ListCheapestInsertionPhase;
pub(crate) use kernel::run_cheapest;
#[cfg(test)]
pub(crate) use kernel::{CheapestInsertionObserver, CheapestInsertionTrial};
pub(crate) use live::PhaseCheapestInsertionObserver;

#[cfg(test)]
#[path = "cheapest/tests.rs"]
mod tests;
