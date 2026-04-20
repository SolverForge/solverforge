/* Solver-facing solution traits.

Shadow updates are part of `PlanningSolution` itself: solutions override
`update_entity_shadows()` and `update_all_shadows()` when they maintain
derived or cached state. `ScoreDirector` calls those hooks directly on the
solution, so the canonical scoring path stays fully monomorphized.
*/

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

/* Trait for solutions that can be solved using the fluent builder API.

This trait combines the remaining requirements for automatic solver wiring:
- `PlanningSolution` for score and shadow lifecycle management
- `SolutionDescriptor` for entity metadata
- entity counting for move selector iteration

Typically implemented automatically by the `#[planning_solution]` macro.
*/
pub trait SolvableSolution: PlanningSolution {
    /* Returns the solution descriptor for this type. */
    fn descriptor() -> SolutionDescriptor;

    /* Returns the entity count for a given descriptor index. */
    fn entity_count(solution: &Self, descriptor_index: usize) -> usize;
}
