/* Shadow variable support traits for solutions with derived values.

Provides [`ShadowVariableSupport`] and [`SolvableSolution`] traits
for integrating shadow variable updates into the solving protocol.
*/

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

/* Trait for solutions that maintain shadow variables.

Shadow variables are derived values that depend on planning variables.
When a planning variable changes, the corresponding shadow variables
must be updated before constraint evaluation.

# Entity-Level Updates

This trait provides entity-level granularity: when a variable on entity N
changes, only entity N's shadow variables are updated. This enables O(1)
incremental updates instead of full solution recalculation.

# Example

```
use solverforge_scoring::director::ShadowVariableSupport;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct Visit {
demand: i32,
// Shadow variable: arrival time depends on previous visit
arrival_time: i64,
}

#[derive(Clone)]
struct Vehicle {
visits: Vec<usize>,
// Cached aggregate: total demand of assigned visits
cached_total_demand: i32,
}

#[derive(Clone)]
struct VrpSolution {
visits: Vec<Visit>,
vehicles: Vec<Vehicle>,
score: Option<SoftScore>,
}

impl PlanningSolution for VrpSolution {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

impl ShadowVariableSupport for VrpSolution {
fn update_entity_shadows(&mut self, entity_index: usize) {
// Update cached total demand for this vehicle
let total: i32 = self.vehicles[entity_index]
.visits
.iter()
.map(|&idx| self.visits[idx].demand)
.sum();
self.vehicles[entity_index].cached_total_demand = total;
}

fn update_all_shadows(&mut self) {
for i in 0..self.vehicles.len() {
self.update_entity_shadows(i);
}
}
}
```
*/
pub trait ShadowVariableSupport: PlanningSolution {
    /* Updates shadow variables for the entity at `entity_index`.

    Called after a planning variable change on this entity, before
    constraint evaluation. Should update all shadow variables and
    cached aggregates that depend on this entity's planning variables.
    */
    fn update_entity_shadows(&mut self, entity_index: usize);

    /* Updates shadow variables for all entities.

    Called during initialization or after bulk solution changes.
    Default implementation is a no-op; override for solutions with
    shadow variables.
    */
    fn update_all_shadows(&mut self) {
        // Default: no-op - solutions without shadow variables need not implement
    }
}

/* Trait for solutions that can be solved using the fluent builder API.

This trait combines all requirements for automatic solver wiring:
- `PlanningSolution` for score management
- `ShadowVariableSupport` for shadow variable updates
- Solution descriptor for entity metadata
- Entity count for move selector iteration

Typically implemented automatically by the `#[planning_solution]` macro.

# Example

```
use solverforge_scoring::ShadowVariableSupport;
use solverforge_scoring::director::SolvableSolution;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use std::any::TypeId;

#[derive(Clone)]
struct MyPlan {
entities: Vec<i64>,
score: Option<SoftScore>,
}

impl PlanningSolution for MyPlan {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

impl ShadowVariableSupport for MyPlan {
fn update_entity_shadows(&mut self, _idx: usize) {}
}

impl SolvableSolution for MyPlan {
fn descriptor() -> SolutionDescriptor {
SolutionDescriptor::new("MyPlan", TypeId::of::<MyPlan>())
}
fn entity_count(solution: &Self, _desc_idx: usize) -> usize {
solution.entities.len()
}
}
```
*/
pub trait SolvableSolution: ShadowVariableSupport {
    /* Returns the solution descriptor for this type.

    The descriptor provides entity metadata for the solver infrastructure.
    */
    fn descriptor() -> SolutionDescriptor;

    /* Returns the entity count for a given descriptor index.

    This is an associated function (not a method) to match the
    `fn(&S, usize) -> usize` signature required by `ScoreDirector`.
    */
    fn entity_count(solution: &Self, descriptor_index: usize) -> usize;
}
