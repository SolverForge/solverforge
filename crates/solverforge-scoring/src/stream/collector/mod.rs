//! Collectors for grouping and aggregating entities.
//!
//! Collectors aggregate entities within groups during `group_by()` operations.
//! They maintain incremental state for O(1) insert/retract operations.
//!
//! # Example
//!
//! ```
//! use solverforge_scoring::stream::collector::{count, sum};
//!
//! // Count collector for counting entities in a group
//! let counter = count::<i32>();
//!
//! // Sum collector for summing a property
//! let summer = sum(|x: &i32| *x as i64);
//! ```

mod count;
mod load_balance;
mod sum;

#[cfg(test)]
mod tests;

pub use count::{count, CountAccumulator, CountCollector};
pub use load_balance::{load_balance, LoadBalance, LoadBalanceAccumulator, LoadBalanceCollector};
pub use sum::{sum, SumAccumulator, SumCollector};

/// A collector that aggregates entities of type `A` into a result of type `R`.
///
/// Collectors are used in `group_by()` operations to reduce groups of entities
/// into summary values.
///
/// # Zero-Erasure Design
///
/// The collector owns any mapping functions and provides `extract()` to convert
/// entities to values. The accumulator only works with extracted values, avoiding
/// the need to clone mapping functions into each accumulator.
///
/// # Incremental Protocol
///
/// Collectors support incremental updates:
/// 1. `create_accumulator()` creates a fresh accumulator
/// 2. `extract(entity)` converts entity to accumulator value
/// 3. `accumulate(value)` adds value to accumulator
/// 4. `retract(value)` removes value from accumulator
/// 5. `finish()` produces the final result
///
/// This enables O(1) score updates when entities are added/removed from groups.
pub trait UniCollector<A>: Send + Sync {
    /// The value type extracted from entities and passed to the accumulator.
    type Value;

    /// The result type produced by this collector.
    type Result: Clone + Send + Sync;

    /// The accumulator type used during collection.
    type Accumulator: Accumulator<Self::Value, Self::Result>;

    /// Extracts the value to accumulate from an entity.
    fn extract(&self, entity: &A) -> Self::Value;

    /// Creates a fresh accumulator.
    fn create_accumulator(&self) -> Self::Accumulator;
}

/// An accumulator that incrementally collects values.
///
/// Values are extracted by the collector's `extract()` method before
/// being passed to the accumulator by reference.
pub trait Accumulator<V, R>: Send + Sync {
    /// Adds a value to the accumulator.
    fn accumulate(&mut self, value: &V);

    /// Removes a value from the accumulator.
    fn retract(&mut self, value: &V);

    /// Produces the final result.
    fn finish(&self) -> R;

    /// Resets the accumulator to its initial state.
    fn reset(&mut self);
}
