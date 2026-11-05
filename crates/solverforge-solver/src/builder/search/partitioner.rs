//! Partitioners registered on a search declaration by name, for the
//! framework's `partitioned_search` phase.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::phase::partitioned::SolutionPartitioner;

use super::custom::{RuntimeExtensionPolicy, RuntimeExtensionRegistry};
use super::SearchContext;

/// The partitioner of a registry with none registered.
#[derive(Clone, Copy)]
pub enum NoPartitioner {}

impl Debug for NoPartitioner {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl<S: PlanningSolution> SolutionPartitioner<S> for NoPartitioner {
    fn partition(&self, _solution: &S) -> Vec<S> {
        match *self {}
    }

    fn merge(&self, _original: &S, _partitions: Vec<S>) -> S {
        match *self {}
    }
}

/// One registered partitioner: `builder` makes it from the solve's
/// [`SearchContext`] when a `partitioned_search` phase names it.
pub struct PartitionerNode<Previous, Builder, Partitioner> {
    previous: Previous,
    name: &'static str,
    builder: Builder,
    _marker: PhantomData<fn() -> Partitioner>,
}

impl<Previous, Builder, Partitioner> PartitionerNode<Previous, Builder, Partitioner> {
    pub fn new(previous: Previous, name: &'static str, builder: Builder) -> Self {
        Self {
            previous,
            name,
            builder,
            _marker: PhantomData,
        }
    }
}

/// Either a previously registered partitioner or the current one.
pub enum PartitionerUnion<Previous, Current> {
    Previous(Previous),
    Current(Current),
}

impl<Previous: Debug, Current: Debug> Debug for PartitionerUnion<Previous, Current> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Previous(partitioner) => f
                .debug_tuple("Partitioner::Previous")
                .field(partitioner)
                .finish(),
            Self::Current(partitioner) => f
                .debug_tuple("Partitioner::Current")
                .field(partitioner)
                .finish(),
        }
    }
}

impl<S, Previous, Current> SolutionPartitioner<S> for PartitionerUnion<Previous, Current>
where
    S: PlanningSolution,
    Previous: SolutionPartitioner<S>,
    Current: SolutionPartitioner<S>,
{
    fn partition(&self, solution: &S) -> Vec<S> {
        match self {
            Self::Previous(partitioner) => partitioner.partition(solution),
            Self::Current(partitioner) => partitioner.partition(solution),
        }
    }

    fn merge(&self, original: &S, partitions: Vec<S>) -> S {
        match self {
            Self::Previous(partitioner) => partitioner.merge(original, partitions),
            Self::Current(partitioner) => partitioner.merge(original, partitions),
        }
    }

    fn recommended_partition_count(&self) -> Option<usize> {
        match self {
            Self::Previous(partitioner) => partitioner.recommended_partition_count(),
            Self::Current(partitioner) => partitioner.recommended_partition_count(),
        }
    }
}

impl<S, V, DM, IDM, Previous, Builder, Current> RuntimeExtensionRegistry<S, V, DM, IDM>
    for PartitionerNode<Previous, Builder, Current>
where
    S: PlanningSolution,
    Previous: RuntimeExtensionRegistry<S, V, DM, IDM>,
    Builder: Fn(&SearchContext<S, V, DM, IDM>) -> Current + Send + Sync,
    Current: SolutionPartitioner<S>,
{
    type Phase = Previous::Phase;
    type Partitioner = PartitionerUnion<Previous::Partitioner, Current>;

    fn policy(&self) -> RuntimeExtensionPolicy {
        self.previous.policy()
    }

    fn contains_custom(&self, name: &str) -> bool {
        self.previous.contains_custom(name)
    }

    fn instantiate_custom(
        &self,
        name: &str,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Phase> {
        self.previous.instantiate_custom(name, context)
    }

    fn contains_partitioner(&self, name: &str) -> bool {
        self.name == name || self.previous.contains_partitioner(name)
    }

    fn instantiate_partitioner(
        &self,
        name: &str,
        context: &SearchContext<S, V, DM, IDM>,
    ) -> Option<Self::Partitioner> {
        if self.name == name {
            return Some(PartitionerUnion::Current((self.builder)(context)));
        }
        self.previous
            .instantiate_partitioner(name, context)
            .map(PartitionerUnion::Previous)
    }
}
