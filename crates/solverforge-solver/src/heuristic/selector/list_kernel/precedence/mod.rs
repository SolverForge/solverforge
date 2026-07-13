//! Canonical critical-path precedence neighborhood.

mod analysis;
mod coordinates;
mod cursor;
mod emission;
mod support;

pub(crate) use analysis::{critical_analysis, critical_analysis_from_graph, CriticalAnalysis};
pub(crate) use coordinates::filtered_move_count;
pub(crate) use cursor::PrecedenceCursor;
pub(crate) use emission::{NativePrecedenceEmitter, PrecedenceEmitter};
pub(crate) use support::{filtered_multi_support_swap_count, multi_critical_ruin_count};
