// Collectors for grouping and aggregating stream matches.

mod collect_vec;
mod core;
mod count;
mod indexed_presence;
mod load_balance;
mod runs;
mod sum;

#[cfg(test)]
mod tests;

pub use collect_vec::{collect_vec, CollectVecAccumulator, CollectVecCollector, CollectedVec};
pub use core::{Accumulator, Collector};
pub use count::{count, CountAccumulator, CountCollector};
pub use indexed_presence::{
    indexed_presence, IndexedPresence, IndexedPresenceAccumulator, IndexedPresenceCollector,
};
pub use load_balance::{load_balance, LoadBalance, LoadBalanceAccumulator, LoadBalanceCollector};
pub use runs::{consecutive_runs, Run, Runs, RunsAccumulator, RunsCollector};
pub use sum::{sum, SumAccumulator, SumCollector};
