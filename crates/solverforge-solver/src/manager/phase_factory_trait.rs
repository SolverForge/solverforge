//! PhaseFactory trait definition.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;

/// Factory trait for creating phases with zero type erasure.
///
/// Returns a concrete phase type via associated type, preserving
/// full type information through the pipeline.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `D` - The score director type
pub trait PhaseFactory<S, D>: Send + Sync
where
    S: PlanningSolution,
    D: Director<S>,
{
    /// The concrete phase type produced by this factory.
    type Phase: Phase<S, D>;

    /// Creates a new phase instance with concrete type.
    fn create(&self) -> Self::Phase;
}
