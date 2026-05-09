// Collectors for grouping and aggregating entities.

mod collect_vec;
mod count;
mod load_balance;
mod runs;
mod sum;
mod uni;

#[cfg(test)]
mod tests;

pub use collect_vec::{collect_vec, CollectVecAccumulator, CollectVecCollector};
pub use count::{count, CountAccumulator, CountCollector};
pub use load_balance::{load_balance, LoadBalance, LoadBalanceAccumulator, LoadBalanceCollector};
pub use runs::{consecutive_runs, Run, Runs, RunsAccumulator, RunsCollector};
pub use sum::{sum, SumAccumulator, SumCollector};
pub use uni::{Accumulator, UniCollector};
