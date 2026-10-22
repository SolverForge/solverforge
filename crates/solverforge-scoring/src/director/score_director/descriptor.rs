use std::marker::PhantomData;

use solverforge_core::domain::SolutionDescriptor;

/* How a `ScoreDirector` holds its solution descriptor.

An owned `SolutionDescriptor` serves a single director. A
`ReplicaDescriptor` borrows the descriptor and carries the constraint-set
constructor, so the director can fork evaluator replicas that share the same
descriptor without copying it.
*/
pub trait DirectorDescriptor<S, C> {
    fn solution_descriptor(&self) -> &SolutionDescriptor;

    /* A handle for an evaluator replica plus the constructor that builds the
    replica's constraint set from its own working solution. `None` when
    this director cannot fork replicas.
    */
    fn replica(&self) -> Option<(Self, fn(&S) -> C)>
    where
        Self: Sized;
}

impl<S, C> DirectorDescriptor<S, C> for SolutionDescriptor {
    #[inline]
    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self
    }

    fn replica(&self) -> Option<(Self, fn(&S) -> C)> {
        None
    }
}

/* A borrowed solution descriptor plus the constraint-set constructor used for
the working solution and every evaluator replica.
*/
pub struct ReplicaDescriptor<'d, S, C> {
    descriptor: &'d SolutionDescriptor,
    constraints: fn(&S) -> C,
    _phantom: PhantomData<fn() -> (S, C)>,
}

impl<'d, S, C> ReplicaDescriptor<'d, S, C> {
    pub fn new(descriptor: &'d SolutionDescriptor, constraints: fn(&S) -> C) -> Self {
        Self {
            descriptor,
            constraints,
            _phantom: PhantomData,
        }
    }
}

impl<S, C> Clone for ReplicaDescriptor<'_, S, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, C> Copy for ReplicaDescriptor<'_, S, C> {}

impl<S, C> DirectorDescriptor<S, C> for ReplicaDescriptor<'_, S, C> {
    #[inline]
    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self.descriptor
    }

    fn replica(&self) -> Option<(Self, fn(&S) -> C)> {
        Some((*self, self.constraints))
    }
}
