//! Shared runtime-list construction kernels.
//!
//! List construction enters through a compiled RuntimeListSlot. The same
//! enumeration kernel is used by live solving and detached preview observers;
//! physical native/dynamic storage never selects a different algorithm.

mod access;
mod clarke_wright;
mod k_opt;
mod phase;
mod regret;
mod round_robin;

#[cfg(test)]
mod cheapest_tests;
#[cfg(test)]
mod k_opt_tests;
#[cfg(test)]
mod source_tests;

pub(crate) use clarke_wright::execute_runtime_list_clarke_wright;
pub(crate) use k_opt::execute_runtime_list_k_opt;
pub(crate) use phase::execute_runtime_list_cheapest_insertion;
pub(crate) use regret::execute_runtime_list_regret_insertion;
pub(crate) use round_robin::execute_runtime_list_round_robin;
