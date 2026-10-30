//! One fallible lowering from frozen selector graph nodes to retained state.

use std::fmt;

use solverforge_config::{MoveSelectorConfig, SelectionOrder, SolverConfig, UnionSelectionOrder};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::builder::selector::types::{
    CompositionNodeId, SelectorComposition, SelectorCompositionBorrowedCursor,
    SelectorCompositionNode, SelectorCompositionState,
};
use crate::builder::RuntimeCandidateMetricBinding;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::scalar_neighborhood::{
    ScalarNeighborhoodKind, ScalarNeighborhoodLeaf,
};
use crate::runtime::compiler::graph::{
    CompiledSelectorGraph, CompiledSelectorNode, CompiledSelectorNodeId, ScalarLeafKind,
};

use super::super::CompiledListNeighborhoodLeafAdapter;
use super::leaf::{
    ProviderExecutionResources, RuntimeNeighborhoodFlat, RuntimeNeighborhoodLeaf,
    RuntimeNeighborhoodLeafStreamState, RuntimeProviderNeighborhoodLeaf,
};
use super::r#move::RuntimeNeighborhoodMove;

pub(crate) type RuntimeNeighborhoodComposition<S, V, DM, IDM> = SelectorComposition<
    S,
    RuntimeNeighborhoodMove<S, V, DM, IDM>,
    RuntimeNeighborhoodFlat<S, V, DM, IDM>,
    Vec<RuntimeNeighborhoodLeafStreamState>,
>;

/// The live cursor a compiled local-search step pulls candidates from.
pub(crate) type RuntimeNeighborhoodCursor<'a, S, V, DM, IDM> = SelectorCompositionBorrowedCursor<
    'a,
    S,
    RuntimeNeighborhoodMove<S, V, DM, IDM>,
    RuntimeNeighborhoodFlat<S, V, DM, IDM>,
    Vec<RuntimeNeighborhoodLeafStreamState>,
    ProviderExecutionResources<S>,
>;

pub(crate) type RuntimeNeighborhoodState<S, V, DM, IDM> = SelectorCompositionState<
    S,
    RuntimeNeighborhoodMove<S, V, DM, IDM>,
    RuntimeNeighborhoodFlat<S, V, DM, IDM>,
    Vec<RuntimeNeighborhoodLeafStreamState>,
    ProviderExecutionResources<S>,
>;

/// A frozen graph/leaf handoff failed while instantiating one solve. It is not
/// a compile-time schema error and never selects an alternate selector path.
#[derive(Debug)]
pub(crate) struct RuntimeLocalSearchLoweringError {
    message: String,
}

impl RuntimeLocalSearchLoweringError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RuntimeLocalSearchLoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RuntimeLocalSearchLoweringError {}

/// Consumes one child node out of the graph arena in postorder; structural
/// postorder guarantees every arena slot lowers exactly once.
fn pop_child<S, V, DM, IDM>(
    nodes: &mut [Option<CompiledSelectorNode<S, V, DM, IDM>>],
    id: CompiledSelectorNodeId,
) -> CompiledSelectorNode<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    nodes[id.index()]
        .take()
        .expect("each compiled selector node lowers exactly once")
}

/// Lowers one already-compiled graph to retained per-solve stream state,
/// consuming the graph. Every branch receives frozen node data by ownership;
/// it never scans a model, rebuilds a public selector, invokes a provider
/// callback, or clones a slot vector or provider plan.
pub(crate) fn lower_selector<S, V, DM, IDM>(
    solver_config: &SolverConfig,
    graph: CompiledSelectorGraph<S, V, DM, IDM>,
) -> Result<RuntimeNeighborhoodState<S, V, DM, IDM>, RuntimeLocalSearchLoweringError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let (mut nodes, root) = graph.into_parts();
    let root_node = pop_child(&mut nodes, root);
    let mut composition = SelectorComposition::new();
    lower_node(solver_config, &mut nodes, root_node, &mut composition)?;
    Ok(SelectorCompositionState::new(composition))
}

/// Lowers the exact frozen default declaration vector as one root union.
/// `DefaultRuntimeBindings` owns the vector's provenance; this helper merely
/// consumes it in that preserved order and never recompiles a selector.
pub(crate) fn lower_default_selector_union<S, V, DM, IDM>(
    solver_config: &SolverConfig,
    selection_order: UnionSelectionOrder,
    graphs: Vec<CompiledSelectorGraph<S, V, DM, IDM>>,
) -> Result<RuntimeNeighborhoodState<S, V, DM, IDM>, RuntimeLocalSearchLoweringError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    if graphs.is_empty() {
        return Err(RuntimeLocalSearchLoweringError::new(
            "default local-search policy retained no frozen selector nodes",
        ));
    }
    let mut composition = SelectorComposition::new();
    let children = graphs
        .into_iter()
        .map(|graph| {
            let (mut nodes, root) = graph.into_parts();
            let root_node = pop_child(&mut nodes, root);
            lower_node(solver_config, &mut nodes, root_node, &mut composition)
        })
        .collect::<Result<Vec<_>, _>>()?;
    composition.push(SelectorCompositionNode::Union {
        selection_order,
        weighting: solverforge_config::UnionWeighting::Equal,
        weights: Vec::new(),
        children,
    });
    Ok(SelectorCompositionState::new(composition))
}

fn lower_node<S, V, DM, IDM>(
    solver_config: &SolverConfig,
    nodes: &mut [Option<CompiledSelectorNode<S, V, DM, IDM>>],
    node: CompiledSelectorNode<S, V, DM, IDM>,
    arena: &mut RuntimeNeighborhoodComposition<S, V, DM, IDM>,
) -> Result<CompositionNodeId, RuntimeLocalSearchLoweringError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let declaration = match node {
        CompiledSelectorNode::Scalar {
            kind,
            config,
            candidate_order,
            candidate_metric,
            slots,
        } => {
            let leaves = slots
                .into_iter()
                .map(|slot| {
                    ScalarNeighborhoodLeaf::new(
                        scalar_neighborhood_kind(kind),
                        &config,
                        slot,
                        solver_config.random_seed,
                    )
                    .map(RuntimeNeighborhoodLeaf::Scalar)
                    .map_err(|error| {
                        RuntimeLocalSearchLoweringError::new(format!(
                            "compiled scalar neighborhood leaf could not instantiate: {error}"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(flat(leaves, candidate_order, candidate_metric)?)
        }
        CompiledSelectorNode::List {
            kind,
            config,
            candidate_order,
            candidate_metric,
            slots,
        } => {
            let leaf = CompiledListNeighborhoodLeafAdapter::from_solver_config(solver_config)
                .lower_compiled_list(kind, config, slots)
                .map_err(|error| {
                    RuntimeLocalSearchLoweringError::new(format!(
                        "compiled list neighborhood leaf could not instantiate: {error}"
                    ))
                })?;
            Ok(flat(
                vec![RuntimeNeighborhoodLeaf::List(leaf)],
                candidate_order,
                candidate_metric,
            )?)
        }
        CompiledSelectorNode::GroupedScalar {
            config,
            candidate_order,
            candidate_metric,
            group,
            ..
        } => {
            let MoveSelectorConfig::GroupedScalarMoveSelector(grouped) = config else {
                return Err(RuntimeLocalSearchLoweringError::new(
                    "compiled grouped scalar node retained an incompatible selector configuration",
                ));
            };
            Ok(flat(
                vec![RuntimeNeighborhoodLeaf::Grouped(
                    crate::builder::selector::GroupedScalarSelector::new(
                        group,
                        grouped.value_candidate_limit,
                        grouped.max_moves_per_step,
                        grouped.require_hard_improvement,
                    ),
                )],
                candidate_order,
                candidate_metric,
            )?)
        }
        CompiledSelectorNode::Provider {
            config,
            candidate_order,
            candidate_metric,
            plan,
        } => Ok(flat(
            vec![RuntimeNeighborhoodLeaf::Provider(
                RuntimeProviderNeighborhoodLeaf::new(
                    plan,
                    provider_requires_hard_improvement(&config)?,
                ),
            )],
            candidate_order,
            candidate_metric,
        )?),
        CompiledSelectorNode::Limited {
            selected_count_limit,
            selector,
        } => {
            let child_node = pop_child(nodes, selector);
            let child = lower_node(solver_config, nodes, child_node, arena)?;
            Ok(SelectorCompositionNode::Limited {
                selector: child,
                selected_count_limit,
            })
        }
        CompiledSelectorNode::Union {
            selection_order,
            weighting,
            weights,
            children,
        } => {
            if children.is_empty() {
                return Err(RuntimeLocalSearchLoweringError::new(
                    "compiled union retained no frozen selector children",
                ));
            }
            Ok(SelectorCompositionNode::Union {
                selection_order,
                weighting,
                weights,
                children: children
                    .into_iter()
                    .map(|child| {
                        let child_node = pop_child(nodes, child);
                        lower_node(solver_config, nodes, child_node, arena)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
        CompiledSelectorNode::Cartesian {
            require_hard_improvement,
            left,
            right,
        } => {
            let left_node = pop_child(nodes, left);
            let right_node = pop_child(nodes, right);
            let left = lower_node(solver_config, nodes, left_node, arena)?;
            let right = lower_node(solver_config, nodes, right_node, arena)?;
            Ok(SelectorCompositionNode::Cartesian {
                require_hard_improvement,
                left,
                right,
            })
        }
    }?;
    Ok(arena.push(declaration))
}

fn scalar_neighborhood_kind(kind: ScalarLeafKind) -> ScalarNeighborhoodKind {
    match kind {
        ScalarLeafKind::Change => ScalarNeighborhoodKind::Change,
        ScalarLeafKind::Swap => ScalarNeighborhoodKind::Swap,
        ScalarLeafKind::NearbyChange => ScalarNeighborhoodKind::NearbyChange,
        ScalarLeafKind::NearbySwap => ScalarNeighborhoodKind::NearbySwap,
        ScalarLeafKind::PillarChange => ScalarNeighborhoodKind::PillarChange,
        ScalarLeafKind::PillarSwap => ScalarNeighborhoodKind::PillarSwap,
        ScalarLeafKind::RuinRecreate => ScalarNeighborhoodKind::RuinRecreate,
    }
}

fn flat<S, V, DM, IDM>(
    leaves: Vec<RuntimeNeighborhoodLeaf<S, V, DM, IDM>>,
    candidate_order: SelectionOrder,
    candidate_metric: Option<RuntimeCandidateMetricBinding<S>>,
) -> Result<
    SelectorCompositionNode<RuntimeNeighborhoodFlat<S, V, DM, IDM>>,
    RuntimeLocalSearchLoweringError,
>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    if leaves.is_empty() {
        return Err(RuntimeLocalSearchLoweringError::new(
            "compiled neighborhood leaf retained no matching frozen slots",
        ));
    }
    let selection_order = match candidate_order {
        SelectionOrder::Random | SelectionOrder::Shuffled => UnionSelectionOrder::StratifiedRandom,
        SelectionOrder::Original | SelectionOrder::Sorted | SelectionOrder::Probabilistic => {
            UnionSelectionOrder::Sequential
        }
    };
    Ok(SelectorCompositionNode::Flat(RuntimeNeighborhoodFlat::new(
        leaves,
        selection_order,
        candidate_order,
        candidate_metric,
    )))
}

fn provider_requires_hard_improvement(
    config: &MoveSelectorConfig,
) -> Result<bool, RuntimeLocalSearchLoweringError> {
    match config {
        MoveSelectorConfig::GroupedScalarMoveSelector(config) => {
            Ok(config.require_hard_improvement)
        }
        MoveSelectorConfig::ConflictRepairMoveSelector(config) => {
            Ok(config.require_hard_improvement)
        }
        MoveSelectorConfig::CompoundConflictRepairMoveSelector(config) => {
            Ok(config.require_hard_improvement)
        }
        _ => Err(RuntimeLocalSearchLoweringError::new(
            "compiled provider node retained an incompatible selector configuration",
        )),
    }
}
