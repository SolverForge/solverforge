use solverforge_core::domain::SolutionDescriptor;

/// How a `ScoreDirector` holds its solution descriptor.
///
/// An owned `SolutionDescriptor` serves a single director. A borrowed
/// `&SolutionDescriptor` can be shared, so a director built with
/// `ScoreDirector::with_replicas` forks evaluator replicas that share the same
/// descriptor without copying it.
pub trait DirectorDescriptor {
    /// Returns the solution descriptor.
    fn solution_descriptor(&self) -> &SolutionDescriptor;

    /// The same descriptor for an evaluator replica, or `None` when this
    /// descriptor cannot be shared.
    fn share(&self) -> Option<Self>
    where
        Self: Sized;
}

impl DirectorDescriptor for SolutionDescriptor {
    #[inline]
    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self
    }

    fn share(&self) -> Option<Self> {
        None
    }
}

impl DirectorDescriptor for &SolutionDescriptor {
    #[inline]
    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self
    }

    fn share(&self) -> Option<Self> {
        Some(*self)
    }
}
