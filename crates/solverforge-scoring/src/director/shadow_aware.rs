// Shadow-aware score director for solutions with shadow variables.
//
// Provides [`ShadowVariableSupport`] trait and [`ShadowAwareScoreDirector`]
// that integrates shadow variable updates into the change notification protocol.

use std::any::Any;
use std::marker::PhantomData;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use super::ScoreDirector;

// Trait for solutions that maintain shadow variables.
//
// Shadow variables are derived values that depend on planning variables.
// When a planning variable changes, the corresponding shadow variables
// must be updated before constraint evaluation.
//
// # Entity-Level Updates
//
// This trait provides entity-level granularity: when a variable on entity N
// changes, only entity N's shadow variables are updated. This enables O(1)
// incremental updates instead of full solution recalculation.
//
// # Example
//
// ```
// use solverforge_scoring::director::ShadowVariableSupport;
// use solverforge_core::domain::PlanningSolution;
// use solverforge_core::score::SimpleScore;
//
// #[derive(Clone)]
// struct Visit {
//     demand: i32,
//     // Shadow variable: arrival time depends on previous visit
//     arrival_time: i64,
// }
//
// #[derive(Clone)]
// struct Vehicle {
//     visits: Vec<usize>,
//     // Cached aggregate: total demand of assigned visits
//     cached_total_demand: i32,
// }
//
// #[derive(Clone)]
// struct VrpSolution {
//     visits: Vec<Visit>,
//     vehicles: Vec<Vehicle>,
//     score: Option<SimpleScore>,
// }
//
// impl PlanningSolution for VrpSolution {
//     type Score = SimpleScore;
//     fn score(&self) -> Option<Self::Score> { self.score }
//     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
// }
//
// impl ShadowVariableSupport for VrpSolution {
//     fn update_entity_shadows(&mut self, entity_index: usize) {
//         // Update cached total demand for this vehicle
//         let total: i32 = self.vehicles[entity_index]
//             .visits
//             .iter()
//             .map(|&idx| self.visits[idx].demand)
//             .sum();
//         self.vehicles[entity_index].cached_total_demand = total;
//     }
//
//     fn update_all_shadows(&mut self) {
//         for i in 0..self.vehicles.len() {
//             self.update_entity_shadows(i);
//         }
//     }
// }
// ```
pub trait ShadowVariableSupport: PlanningSolution {
    // Updates shadow variables for the entity at `entity_index`.
    //
    // Called after a planning variable change on this entity, before
    // constraint evaluation. Should update all shadow variables and
    // cached aggregates that depend on this entity's planning variables.
    fn update_entity_shadows(&mut self, entity_index: usize);

    // Updates shadow variables for all entities.
    //
    // Called during initialization or after bulk solution changes.
    // Default implementation is a no-op; override for solutions with
    // shadow variables.
    fn update_all_shadows(&mut self) {
        // Default: no-op - solutions without shadow variables need not implement
    }
}

// Trait for solutions that can be solved using the fluent builder API.
//
// This trait combines all requirements for automatic solver wiring:
// - `PlanningSolution` for score management
// - `ShadowVariableSupport` for shadow variable updates
// - Solution descriptor for entity metadata
// - Entity count for move selector iteration
//
// Typically implemented automatically by the `#[planning_solution]` macro.
//
// # Example
//
// ```
// use solverforge_scoring::ShadowVariableSupport;
// use solverforge_scoring::director::SolvableSolution;
// use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
// use solverforge_core::score::SimpleScore;
// use std::any::TypeId;
//
// #[derive(Clone)]
// struct MyPlan {
//     entities: Vec<i64>,
//     score: Option<SimpleScore>,
// }
//
// impl PlanningSolution for MyPlan {
//     type Score = SimpleScore;
//     fn score(&self) -> Option<Self::Score> { self.score }
//     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
// }
//
// impl ShadowVariableSupport for MyPlan {
//     fn update_entity_shadows(&mut self, _idx: usize) {}
// }
//
// impl SolvableSolution for MyPlan {
//     fn descriptor() -> SolutionDescriptor {
//         SolutionDescriptor::new("MyPlan", TypeId::of::<MyPlan>())
//     }
//     fn entity_count(solution: &Self, _desc_idx: usize) -> usize {
//         solution.entities.len()
//     }
// }
// ```
pub trait SolvableSolution: ShadowVariableSupport {
    // Returns the solution descriptor for this type.
    //
    // The descriptor provides entity metadata for the solver infrastructure.
    fn descriptor() -> SolutionDescriptor;

    // Returns the entity count for a given descriptor index.
    //
    // This is an associated function (not a method) to match the
    // `fn(&S, usize) -> usize` signature required by `TypedScoreDirector`.
    fn entity_count(solution: &Self, descriptor_index: usize) -> usize;
}

// A score director that integrates shadow variable updates.
//
// Wraps an inner score director and calls [`ShadowVariableSupport::update_entity_shadows`]
// in `after_variable_changed`, ensuring shadow variables are current before
// constraint evaluation.
//
// # Type Parameters
//
// - `S`: Solution type (must implement [`ShadowVariableSupport`])
// - `D`: Inner score director type (zero-erasure, no trait objects)
//
// # Example
//
// ```
// use solverforge_scoring::director::{ShadowAwareScoreDirector, ShadowVariableSupport, ScoreDirector};
// use solverforge_scoring::director::typed::TypedScoreDirector;
// use solverforge_scoring::api::constraint_set::ConstraintSet;
// use solverforge_scoring::constraint::incremental::IncrementalUniConstraint;
// use solverforge_core::domain::PlanningSolution;
// use solverforge_core::{ConstraintRef, ImpactType};
// use solverforge_core::score::SimpleScore;
//
// #[derive(Clone)]
// struct Solution {
//     values: Vec<i32>,
//     // Shadow: sum of all values
//     cached_sum: i32,
//     score: Option<SimpleScore>,
// }
//
// impl PlanningSolution for Solution {
//     type Score = SimpleScore;
//     fn score(&self) -> Option<Self::Score> { self.score }
//     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
// }
//
// impl ShadowVariableSupport for Solution {
//     fn update_entity_shadows(&mut self, _entity_index: usize) {
//         self.cached_sum = self.values.iter().sum();
//     }
// }
//
// // Create constraint that uses cached_sum
// let constraint = IncrementalUniConstraint::new(
//     ConstraintRef::new("", "SumLimit"),
//     ImpactType::Penalty,
//     |s: &Solution| std::slice::from_ref(&s.cached_sum),
//     |_s: &Solution, &sum| sum > 100,
//     |&sum| SimpleScore::of((sum - 100) as i64),
//     false,
// );
//
// let solution = Solution { values: vec![10, 20, 30], cached_sum: 0, score: None };
// let inner = TypedScoreDirector::new(solution, (constraint,));
// let mut director = ShadowAwareScoreDirector::new(inner);
//
// // Shadow variables are updated automatically on variable changes
// director.before_variable_changed(0, 0, "values");
// director.working_solution_mut().values[0] = 50;
// director.after_variable_changed(0, 0, "values");
//
// // cached_sum is now 100 (50 + 20 + 30)
// assert_eq!(director.working_solution().cached_sum, 100);
// ```
pub struct ShadowAwareScoreDirector<S, D>
where
    S: ShadowVariableSupport,
    D: ScoreDirector<S>,
{
    inner: D,
    _phantom: PhantomData<S>,
}

impl<S, D> ShadowAwareScoreDirector<S, D>
where
    S: ShadowVariableSupport,
    D: ScoreDirector<S>,
{
    // Creates a new shadow-aware score director wrapping the given inner director.
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    // Returns a reference to the inner score director.
    pub fn inner(&self) -> &D {
        &self.inner
    }

    // Returns a mutable reference to the inner score director.
    pub fn inner_mut(&mut self) -> &mut D {
        &mut self.inner
    }

    // Consumes self and returns the inner score director.
    pub fn into_inner(self) -> D {
        self.inner
    }
}

use crate::api::constraint_set::ConstraintSet;
use crate::director::typed::TypedScoreDirector;
use solverforge_core::score::Score;

impl<S, C> ShadowAwareScoreDirector<S, TypedScoreDirector<S, C>>
where
    S: ShadowVariableSupport,
    S::Score: Score,
    C: ConstraintSet<S, S::Score> + Send,
{
    // Returns constraint match totals for score analysis.
    //
    // Returns a vector of (name, weight, score, match_count) tuples.
    pub fn constraint_match_totals(&self) -> Vec<(String, S::Score, S::Score, usize)> {
        self.inner.constraint_match_totals()
    }
}

impl<S, D> ScoreDirector<S> for ShadowAwareScoreDirector<S, D>
where
    S: ShadowVariableSupport,
    D: ScoreDirector<S>,
{
    fn working_solution(&self) -> &S {
        self.inner.working_solution()
    }

    fn working_solution_mut(&mut self) -> &mut S {
        self.inner.working_solution_mut()
    }

    fn calculate_score(&mut self) -> S::Score {
        self.inner.calculate_score()
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self.inner.solution_descriptor()
    }

    fn clone_working_solution(&self) -> S {
        self.inner.clone_working_solution()
    }

    fn before_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &str,
    ) {
        self.inner
            .before_variable_changed(descriptor_index, entity_index, variable_name);
    }

    fn after_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &str,
    ) {
        // Update shadow variables FIRST
        self.inner
            .working_solution_mut()
            .update_entity_shadows(entity_index);

        // Then notify inner director (constraint evaluation)
        self.inner
            .after_variable_changed(descriptor_index, entity_index, variable_name);
    }

    fn trigger_variable_listeners(&mut self) {
        self.inner.trigger_variable_listeners();
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.inner.entity_count(descriptor_index)
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.inner.total_entity_count()
    }

    fn get_entity(&self, descriptor_index: usize, entity_index: usize) -> Option<&dyn Any> {
        self.inner.get_entity(descriptor_index, entity_index)
    }

    fn is_incremental(&self) -> bool {
        self.inner.is_incremental()
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn register_undo(&mut self, undo: Box<dyn FnOnce(&mut S) + Send>) {
        self.inner.register_undo(undo);
    }
}

impl<S, D> std::fmt::Debug for ShadowAwareScoreDirector<S, D>
where
    S: ShadowVariableSupport + std::fmt::Debug,
    D: ScoreDirector<S> + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShadowAwareScoreDirector")
            .field("inner", &self.inner)
            .finish()
    }
}
