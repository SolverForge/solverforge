use std::fmt::Debug;
use std::hash::Hash;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::MoveSelector;
use crate::phase::localsearch::SelectorCursorSource;
use crate::phase::localsearch::{Acceptor, LocalSearchForager, LocalSearchPhase};
use crate::RuntimeBuildResult;

use super::RuntimeModel;

mod custom;

pub use custom::{
    CustomPhaseNode, CustomSearchPhase, NoDynamicExtensions, NoRuntimeExtensionPhase,
    NoTypedExtensions, PartitionedPhaseNode, RuntimeExtensionPolicy, RuntimeExtensionRegistry,
};

pub struct SearchContext<
    S,
    V = usize,
    DM = crate::heuristic::selector::DefaultCrossEntityDistanceMeter,
    IDM = crate::heuristic::selector::DefaultCrossEntityDistanceMeter,
> where
    S: PlanningSolution,
{
    descriptor: SolutionDescriptor,
    model: RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
}

impl<S, V, DM, IDM> SearchContext<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    pub fn new(
        descriptor: SolutionDescriptor,
        model: RuntimeModel<S, V, DM, IDM>,
        random_seed: Option<u64>,
    ) -> Self {
        Self::try_new(descriptor, model, random_seed).unwrap_or_else(|error| match error {
            crate::RuntimeBuildError::Declaration { message } => panic!("{message}"),
            other => panic!("{other}"),
        })
    }

    pub fn try_new(
        descriptor: SolutionDescriptor,
        model: RuntimeModel<S, V, DM, IDM>,
        random_seed: Option<u64>,
    ) -> RuntimeBuildResult<Self> {
        let model = model
            .resolve_dynamic_descriptor_indexes(&descriptor)
            .map_err(crate::RuntimeBuildError::declaration)?;
        Ok(Self {
            descriptor,
            model,
            random_seed,
        })
    }

    pub fn descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    pub fn model(&self) -> &RuntimeModel<S, V, DM, IDM> {
        &self.model
    }

    pub fn seed(&self) -> Option<u64> {
        self.random_seed
    }

    pub fn defaults(self) -> SearchBuilder<S, V, DM, IDM, NoTypedExtensions> {
        SearchBuilder {
            context: self,
            extensions: NoTypedExtensions,
        }
    }
}

/// Typed search authoring declaration consumed by the configured runtime.
///
/// Implementations declare one descriptor-resolved runtime model and one
/// concrete typed-extension registry. The configured runner is the sole owner
/// of graph compilation and per-solve phase preparation.
pub trait Search<
    S,
    V = usize,
    DM = crate::heuristic::selector::DefaultCrossEntityDistanceMeter,
    IDM = crate::heuristic::selector::DefaultCrossEntityDistanceMeter,
> where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
{
    /// Concrete typed extension registry transferred with this declaration.
    type Extensions: RuntimeExtensionRegistry<S, V, DM, IDM>;

    /// Transfers descriptor-resolved authoring to the configured runtime.
    ///
    /// This intentionally exposes declaration parts rather than the private
    /// compiled graph. Bindings cannot construct, inspect, cache, or replace
    /// runtime compiler input or prepared phases.
    #[doc(hidden)]
    fn into_runtime_parts(self) -> (SearchContext<S, V, DM, IDM>, Self::Extensions);
}

pub struct SearchBuilder<S, V, DM, IDM, Extensions>
where
    S: PlanningSolution,
{
    context: SearchContext<S, V, DM, IDM>,
    extensions: Extensions,
}

impl<S, V, DM, IDM, Extensions> SearchBuilder<S, V, DM, IDM, Extensions>
where
    S: PlanningSolution,
{
    /// Transfers this typed authoring declaration without building phases.
    pub fn into_runtime_parts(self) -> (SearchContext<S, V, DM, IDM>, Extensions) {
        (self.context, self.extensions)
    }
}

impl<S, V, DM, IDM, Extensions> SearchBuilder<S, V, DM, IDM, Extensions>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    Extensions: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    pub fn phase<P, F>(
        self,
        name: &'static str,
        builder: F,
    ) -> SearchBuilder<S, V, DM, IDM, CustomPhaseNode<Extensions, F, P>>
    where
        F: Fn(&SearchContext<S, V, DM, IDM>) -> P + Send + Sync + 'static,
        P: CustomSearchPhase<S> + 'static,
    {
        assert!(
            !self.extensions.contains_custom(name) && !self.extensions.contains_partitioned(name),
            "custom phase `{name}` was registered more than once",
        );
        SearchBuilder {
            context: self.context,
            extensions: CustomPhaseNode::new(self.extensions, name, builder),
        }
    }

    pub fn partitioned_phase<P, F>(
        self,
        name: &'static str,
        builder: F,
    ) -> SearchBuilder<S, V, DM, IDM, PartitionedPhaseNode<Extensions, F, P>>
    where
        F: Fn(&SearchContext<S, V, DM, IDM>, &solverforge_config::PartitionedSearchConfig) -> P
            + Send
            + Sync
            + 'static,
        P: CustomSearchPhase<S> + 'static,
    {
        assert!(
            !self.extensions.contains_custom(name) && !self.extensions.contains_partitioned(name),
            "partitioned_search partitioner `{name}` was registered more than once",
        );
        SearchBuilder {
            context: self.context,
            extensions: PartitionedPhaseNode::new(self.extensions, name, builder),
        }
    }
}

impl<S, V, DM, IDM, Extensions> Search<S, V, DM, IDM> for SearchBuilder<S, V, DM, IDM, Extensions>
where
    S: PlanningSolution + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + Send + 'static,
    Extensions: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    type Extensions = Extensions;

    fn into_runtime_parts(self) -> (SearchContext<S, V, DM, IDM>, Self::Extensions) {
        SearchBuilder::into_runtime_parts(self)
    }
}

pub fn local_search<S, M, MS, A, Fo>(
    move_selector: MS,
    acceptor: A,
    forager: Fo,
) -> LocalSearchPhase<S, M, SelectorCursorSource<MS>, A, Fo>
where
    S: PlanningSolution,
    M: Move<S> + 'static,
    MS: MoveSelector<S, M>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    LocalSearchPhase::new(move_selector, acceptor, forager, None)
}

#[cfg(test)]
mod tests;
