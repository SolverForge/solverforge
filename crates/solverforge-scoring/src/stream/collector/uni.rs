/* Collector traits for grouping and aggregating entities.

Collectors aggregate entities within groups during `group_by()` operations.
They maintain incremental state for insert/retract operations.
*/

/* A collector that aggregates entities of type `A` into a result of type `R`.

Collectors are used in `group_by()` operations to reduce groups of entities
into summary values.

# Zero-Erasure Design

The collector owns any mapping functions and provides `extract()` to convert
entities to owned values. The accumulator owns retained values and returns
lightweight retraction tokens, avoiding copied or cloned collector payloads in
grouped state.

# Incremental Protocol

Collectors support incremental updates:
1. `create_accumulator()` creates a fresh accumulator
2. `extract(entity)` converts entity to accumulator value
3. `accumulate(value)` moves value into accumulator and returns a retraction token
4. `retract(token)` removes the retained value represented by that token
5. `with_result()` exposes the current result without materializing an owned clone

This enables incremental score updates when entities are added/removed from groups.
*/
pub trait UniCollector<A>: Send + Sync {
    // The value type extracted from entities and passed to the accumulator.
    type Value;

    // The result type produced by this collector.
    type Result: Send + Sync;

    // The accumulator type used during collection.
    type Accumulator: Accumulator<Self::Value, Self::Result>;

    // Extracts the value to accumulate from an entity.
    fn extract(&self, entity: &A) -> Self::Value;

    // Creates a fresh accumulator.
    fn create_accumulator(&self) -> Self::Accumulator;
}

/* An accumulator that incrementally collects values.

Values are extracted by the collector's `extract()` method before being moved
into the accumulator.
*/
pub trait Accumulator<V, R>: Send + Sync {
    // Retained handle needed to undo one accumulated value.
    type Retraction: Send + Sync;

    // Adds an owned value to the accumulator and returns its retraction handle.
    fn accumulate(&mut self, value: V) -> Self::Retraction;

    // Removes a value previously represented by the retraction handle.
    fn retract(&mut self, retraction: Self::Retraction);

    // Exposes the current result without forcing owned result materialization.
    fn with_result<T>(&self, f: impl FnOnce(&R) -> T) -> T;

    // Produces an owned result for cloneable result types.
    fn finish(&self) -> R
    where
        R: Clone,
    {
        self.with_result(Clone::clone)
    }

    // Resets the accumulator to its initial state.
    fn reset(&mut self);
}
