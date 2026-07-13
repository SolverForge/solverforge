//! One fallible lowering from frozen selector graph nodes to retained state.

use std::fmt;

use solverforge_config::{MoveSelectorConfig, SelectionOrder, SolverConfig, UnionSelectionOrder};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::builder::selector::types::{
    SelectorComposition, SelectorCompositionCartesian, SelectorCompositionState,
};
use crate::builder::RuntimeCandidateMetricBinding;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::scalar_neighborhood::{
    ScalarNeighborhoodKind, ScalarNeighborhoodLeaf,
};
use crate::runtime::compiler::graph::{CompiledSelectorNode, ScalarLeafKind};

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

/// Lowers one already-compiled recursive graph to retained per-solve stream
/// state. Every branch consumes frozen node data only; it never scans a model,
/// rebuilds a public selector, or invokes a provider callback.
pub(crate) fn lower_selector<S, V, DM, IDM>(
    solver_config: &SolverConfig,
    node: &CompiledSelectorNode<S, V, DM, IDM>,
) -> Result<RuntimeNeighborhoodState<S, V, DM, IDM>, RuntimeLocalSearchLoweringError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    lower_node(solver_config, node).map(SelectorCompositionState::new)
}

/// Lowers the exact frozen default declaration vector as one root union.
/// `DefaultRuntimeBindings` owns the vector's provenance; this helper merely
/// consumes it in that preserved order and never recompiles a selector.
pub(crate) fn lower_default_selector_union<S, V, DM, IDM>(
    solver_config: &SolverConfig,
    selection_order: UnionSelectionOrder,
    nodes: &[CompiledSelectorNode<S, V, DM, IDM>],
) -> Result<RuntimeNeighborhoodState<S, V, DM, IDM>, RuntimeLocalSearchLoweringError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    if nodes.is_empty() {
        return Err(RuntimeLocalSearchLoweringError::new(
            "default local-search policy retained no frozen selector nodes",
        ));
    }
    let composition = SelectorComposition::Union {
        selection_order,
        weighting: solverforge_config::UnionWeighting::Equal,
        weights: Vec::new(),
        children: nodes
            .iter()
            .map(|node| lower_node(solver_config, node))
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(SelectorCompositionState::new(composition))
}

fn lower_node<S, V, DM, IDM>(
    solver_config: &SolverConfig,
    node: &CompiledSelectorNode<S, V, DM, IDM>,
) -> Result<RuntimeNeighborhoodComposition<S, V, DM, IDM>, RuntimeLocalSearchLoweringError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    match node {
        CompiledSelectorNode::Scalar {
            kind,
            config,
            candidate_order,
            candidate_metric,
            slots,
        } => {
            let leaves = slots
                .iter()
                .cloned()
                .map(|slot| {
                    ScalarNeighborhoodLeaf::new(
                        scalar_neighborhood_kind(*kind),
                        config,
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
            flat(leaves, *candidate_order, candidate_metric.clone())
        }
        CompiledSelectorNode::List {
            candidate_order,
            candidate_metric,
            ..
        } => {
            let leaf = CompiledListNeighborhoodLeafAdapter::from_solver_config(solver_config)
                .lower_compiled_node(node)
                .map_err(|error| {
                    RuntimeLocalSearchLoweringError::new(format!(
                        "compiled list neighborhood leaf could not instantiate: {error}"
                    ))
                })?;
            flat(
                vec![RuntimeNeighborhoodLeaf::List(leaf)],
                *candidate_order,
                candidate_metric.clone(),
            )
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
            flat(
                vec![RuntimeNeighborhoodLeaf::Grouped(
                    crate::builder::selector::GroupedScalarSelector::new(
                        group.clone(),
                        grouped.value_candidate_limit,
                        grouped.max_moves_per_step,
                        grouped.require_hard_improvement,
                    ),
                )],
                *candidate_order,
                candidate_metric.clone(),
            )
        }
        CompiledSelectorNode::Provider {
            config,
            candidate_order,
            candidate_metric,
            plan,
        } => flat(
            vec![RuntimeNeighborhoodLeaf::Provider(
                RuntimeProviderNeighborhoodLeaf::new(
                    plan.clone(),
                    provider_requires_hard_improvement(config)?,
                ),
            )],
            *candidate_order,
            candidate_metric.clone(),
        ),
        CompiledSelectorNode::Limited {
            selected_count_limit,
            selector,
        } => Ok(SelectorComposition::Limited {
            selector: Box::new(lower_node(solver_config, selector)?),
            selected_count_limit: *selected_count_limit,
        }),
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
            Ok(SelectorComposition::Union {
                selection_order: *selection_order,
                weighting: *weighting,
                weights: weights.clone(),
                children: children
                    .iter()
                    .map(|child| lower_node(solver_config, child))
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
        CompiledSelectorNode::Cartesian {
            require_hard_improvement,
            left,
            right,
        } => Ok(SelectorComposition::Cartesian(
            SelectorCompositionCartesian::new(
                lower_node(solver_config, left)?,
                lower_node(solver_config, right)?,
            )
            .with_require_hard_improvement(*require_hard_improvement),
        )),
    }
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
) -> Result<RuntimeNeighborhoodComposition<S, V, DM, IDM>, RuntimeLocalSearchLoweringError>
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
    Ok(SelectorComposition::Flat(RuntimeNeighborhoodFlat::new(
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
