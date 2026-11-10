use std::marker::PhantomData;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;

use super::descriptor::DirectorDescriptor;
use super::incremental::ScoreDirector;
use super::list_shadows::ListShadowRows;
use super::weight_mode::{WeightMode, WeightedMode};
use crate::api::constraint_set::ConstraintSet;

impl<S, C> ScoreDirector<S, C>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Creates a new zero-erasure score director with an empty descriptor.
    ///
    /// Use this for manual solver loops that don't need the `Director` trait.
    /// For full solver infrastructure integration, use `with_descriptor()`.
    ///
    /// The constraints should be a tuple of monomorphized constraints (e.g., `(C1, C2, C3)`).
    pub fn new(solution: S, constraints: C) -> Self {
        use std::any::TypeId;
        Self::with_descriptor(
            solution,
            constraints,
            SolutionDescriptor::new("", TypeId::of::<()>()),
            |_, _| 0,
        )
    }
}

impl<S, C, Desc> ScoreDirector<S, C, Desc>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    Desc: DirectorDescriptor,
{
    /// Creates a new zero-erasure unweighted score director with a solution
    /// descriptor; see `with_descriptor_in_mode`.
    pub fn with_descriptor(
        solution: S,
        constraints: C,
        solution_descriptor: Desc,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        Self::with_descriptor_in_mode(solution, constraints, solution_descriptor, entity_counter)
    }
}

impl<S, C> ScoreDirector<S, C, SolutionDescriptor, WeightedMode>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Creates a weighted score director with an empty descriptor: the
    /// weighted counterpart of `new`.
    pub fn new_weighted(solution: S, constraints: C) -> Self {
        use std::any::TypeId;
        Self::with_descriptor_in_mode(
            solution,
            constraints,
            SolutionDescriptor::new("", TypeId::of::<()>()),
            |_, _| 0,
        )
    }
}

impl<S, C, Desc> ScoreDirector<S, C, Desc, WeightedMode>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    Desc: DirectorDescriptor,
{
    /// Creates a weighted score director with a solution descriptor: the
    /// weighted counterpart of `with_descriptor`.
    pub fn with_descriptor_weighted(
        solution: S,
        constraints: C,
        solution_descriptor: Desc,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        Self::with_descriptor_in_mode(solution, constraints, solution_descriptor, entity_counter)
    }
}

impl<S, C, Desc, M> ScoreDirector<S, C, Desc, M>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    Desc: DirectorDescriptor,
    M: WeightMode,
{
    /// Creates a new zero-erasure score director in weight mode `M` with a
    /// solution descriptor.
    ///
    /// This constructor enables the `Director` trait implementation for
    /// integration with the full solver infrastructure (phases, move selectors, etc.).
    ///
    /// # Arguments
    ///
    /// * `solution` - The initial solution
    /// * `constraints` - Tuple of monomorphized constraints (e.g., `(C1, C2, C3)`)
    /// * `solution_descriptor` - Metadata for solver infrastructure: an owned
    ///   `SolutionDescriptor` or a borrowed one; `with_replicas_in_mode` builds
    ///   a director that forks evaluator replicas
    /// * `entity_counter` - Function returning entity count for descriptor index
    pub fn with_descriptor_in_mode(
        solution: S,
        constraints: C,
        solution_descriptor: Desc,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        let mut weight_mode = M::unbound();
        weight_mode.bind(&solution, &constraints);
        Self {
            working_solution: solution,
            constraints,
            cached_score: S::Score::zero(),
            initialized: false,
            score_assertion: false,
            list_shadow_rows: ListShadowRows::default(),
            weight_mode,
            solution_descriptor,
            constraint_builder: None,
            entity_counter,
            _phantom: PhantomData,
        }
    }

    /// Rebuilds this director in weight mode `M2` over the same working
    /// solution, constraint set and descriptor. Incremental state is dropped as
    /// by `reset()`; the next score calculation initializes under the new mode.
    pub fn into_weight_mode<M2: WeightMode>(mut self) -> ScoreDirector<S, C, Desc, M2> {
        self.reset_impl();
        let mut weight_mode = M2::unbound();
        weight_mode.bind(&self.working_solution, &self.constraints);
        ScoreDirector {
            working_solution: self.working_solution,
            constraints: self.constraints,
            cached_score: self.cached_score,
            initialized: false,
            score_assertion: self.score_assertion,
            list_shadow_rows: self.list_shadow_rows,
            weight_mode,
            solution_descriptor: self.solution_descriptor,
            constraint_builder: self.constraint_builder,
            entity_counter: self.entity_counter,
            _phantom: PhantomData,
        }
    }
}

impl<'d, S, C, M> ScoreDirector<S, C, &'d SolutionDescriptor, M>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    M: WeightMode,
{
    /// Creates a score director in weight mode `M` that forks evaluator
    /// replicas: each replica shares `solution_descriptor` and gets a
    /// constraint set built by `constraint_builder` from its own working
    /// solution. `constraints` is the set for `solution`, normally
    /// `constraint_builder(&solution)`. Reset rebuilds the set the same way,
    /// because constraint sets may compile solution-derived plans that a plain
    /// reset would keep stale.
    pub fn with_replicas_in_mode(
        solution: S,
        constraints: C,
        constraint_builder: fn(&S) -> C,
        solution_descriptor: &'d SolutionDescriptor,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        let mut director = Self::with_descriptor_in_mode(
            solution,
            constraints,
            solution_descriptor,
            entity_counter,
        );
        director.constraint_builder = Some(constraint_builder);
        director
    }
}

impl<'d, S, C> ScoreDirector<S, C, &'d SolutionDescriptor>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Creates an unweighted score director that forks evaluator replicas;
    /// see `with_replicas_in_mode`.
    pub fn with_replicas(
        solution: S,
        constraints: C,
        constraint_builder: fn(&S) -> C,
        solution_descriptor: &'d SolutionDescriptor,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        Self::with_replicas_in_mode(
            solution,
            constraints,
            constraint_builder,
            solution_descriptor,
            entity_counter,
        )
    }
}

impl<'d, S, C> ScoreDirector<S, C, &'d SolutionDescriptor, WeightedMode>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Creates a weighted score director that forks evaluator replicas; see
    /// `with_replicas_in_mode`.
    pub fn with_replicas_weighted(
        solution: S,
        constraints: C,
        constraint_builder: fn(&S) -> C,
        solution_descriptor: &'d SolutionDescriptor,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        Self::with_replicas_in_mode(
            solution,
            constraints,
            constraint_builder,
            solution_descriptor,
            entity_counter,
        )
    }
}

impl<S> ScoreDirector<S, ()>
where
    S: PlanningSolution,
    S::Score: Score,
{
    /// Creates a non-incremental director for use in tests and simple scenarios.
    ///
    /// Uses an empty constraint set — `calculate_score()` always returns zero.
    /// For tests that set score directly on the solution, this is sufficient.
    pub fn simple(
        solution: S,
        descriptor: SolutionDescriptor,
        entity_counter: fn(&S, usize) -> usize,
    ) -> Self {
        Self::with_descriptor(solution, (), descriptor, entity_counter)
    }

    /// Creates a non-incremental director with empty descriptor and zero entity counter.
    pub fn simple_zero(solution: S) -> Self {
        use std::any::TypeId;
        Self::with_descriptor(
            solution,
            (),
            SolutionDescriptor::new("", TypeId::of::<()>()),
            |_, _| 0,
        )
    }
}
