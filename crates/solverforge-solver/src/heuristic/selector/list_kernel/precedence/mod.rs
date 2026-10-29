//! Canonical critical-path precedence neighborhood.

mod analysis;
mod coordinates;
mod cursor;
mod emission;
#[cfg(test)]
mod native_emission;
mod support;

#[cfg(test)]
pub(crate) use analysis::critical_analysis;
pub(crate) use analysis::{critical_analysis_from_graph, CriticalAnalysis};
#[cfg(test)]
pub(crate) use coordinates::filtered_move_count;
pub(crate) use cursor::PrecedenceCursor;
pub(crate) use emission::PrecedenceEmitter;
#[cfg(test)]
pub(crate) use native_emission::NativePrecedenceEmitter;
#[cfg(test)]
pub(crate) use support::{filtered_multi_support_swap_count, multi_critical_ruin_count};
