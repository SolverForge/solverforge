use solverforge_config::{MoveSelectorConfig, SelectionOrder};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::builder::{RuntimeCandidateMetricBinding, RuntimeModel, RuntimeProviderHandle};

use super::graph::{
    rebase_graph, CompiledProviderPlan, CompiledSelectorGraph, CompiledSelectorNode,
    CompiledSelectorNodeId, ListLeafKind, ProviderBindingPlan, ProviderBindingPolicy,
    ProviderMoveKind, ProviderPullTiming, ProviderSchedule, ScalarLeafKind, SelectorGraphBuilder,
    GROUP_PROVIDER_ROTATION_SALT, PYTHON_PROVIDER_CANDIDATE_CONTRACT,
    STATIC_GROUP_PROVIDER_ROTATION_SALT,
};
use super::providers::{
    compile_conflict_repair, scalar_group_provider_slots, unscoped_dynamic_provider_slots,
};
use super::slots::{compile_list_leaf, compile_scalar_leaf};
use super::types::{RuntimeCompileError, RuntimeCompileErrorKind};

mod grouped;
use grouped::{compile_candidate_metric, compile_grouped_or_provider_selector};

pub(super) fn compile_selector<S, V, DM, IDM>(
    config: &MoveSelectorConfig,
    default_candidate_order: SelectionOrder,
    path: &str,
    descriptor: &SolutionDescriptor,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Result<CompiledSelectorGraph<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    let candidate_order = config.selection_order().unwrap_or(default_candidate_order);
    let candidate_metric = compile_candidate_metric(config, candidate_order, path, model)?;
    let mut builder = SelectorGraphBuilder::default();
    let root = match config {
        MoveSelectorConfig::ChangeMoveSelector(selector) => builder.push(compile_scalar_leaf(
            ScalarLeafKind::Change,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        )?),
        MoveSelectorConfig::SwapMoveSelector(selector) => builder.push(compile_scalar_leaf(
            ScalarLeafKind::Swap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        )?),
        MoveSelectorConfig::NearbyChangeMoveSelector(selector) => {
            builder.push(compile_scalar_leaf(
                ScalarLeafKind::NearbyChange,
                config,
                candidate_order,
                candidate_metric.clone(),
                &selector.target,
                path,
                model,
            )?)
        }
        MoveSelectorConfig::NearbySwapMoveSelector(selector) => builder.push(compile_scalar_leaf(
            ScalarLeafKind::NearbySwap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        )?),
        MoveSelectorConfig::PillarChangeMoveSelector(selector) => {
            builder.push(compile_scalar_leaf(
                ScalarLeafKind::PillarChange,
                config,
                candidate_order,
                candidate_metric.clone(),
                &selector.target,
                path,
                model,
            )?)
        }
        MoveSelectorConfig::PillarSwapMoveSelector(selector) => builder.push(compile_scalar_leaf(
            ScalarLeafKind::PillarSwap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            model,
        )?),
        MoveSelectorConfig::RuinRecreateMoveSelector(selector) => {
            builder.push(compile_scalar_leaf(
                ScalarLeafKind::RuinRecreate,
                config,
                candidate_order,
                candidate_metric.clone(),
                &selector.target,
                path,
                model,
            )?)
        }
        MoveSelectorConfig::ListChangeMoveSelector(selector) => builder.push(compile_list_leaf(
            ListLeafKind::Change,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        )?),
        MoveSelectorConfig::NearbyListChangeMoveSelector(selector) => {
            builder.push(compile_list_leaf(
                ListLeafKind::NearbyChange,
                config,
                candidate_order,
                candidate_metric.clone(),
                &selector.target,
                path,
                descriptor,
                model,
            )?)
        }
        MoveSelectorConfig::ListSwapMoveSelector(selector) => builder.push(compile_list_leaf(
            ListLeafKind::Swap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        )?),
        MoveSelectorConfig::ListPermuteMoveSelector(selector) => builder.push(compile_list_leaf(
            ListLeafKind::Permute,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        )?),
        MoveSelectorConfig::ListPrecedenceMoveSelector(selector) => {
            builder.push(compile_list_leaf(
                ListLeafKind::Precedence,
                config,
                candidate_order,
                candidate_metric.clone(),
                &selector.target,
                path,
                descriptor,
                model,
            )?)
        }
        MoveSelectorConfig::NearbyListSwapMoveSelector(selector) => {
            builder.push(compile_list_leaf(
                ListLeafKind::NearbySwap,
                config,
                candidate_order,
                candidate_metric.clone(),
                &selector.target,
                path,
                descriptor,
                model,
            )?)
        }
        MoveSelectorConfig::SublistChangeMoveSelector(selector) => builder.push(compile_list_leaf(
            ListLeafKind::SublistChange,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        )?),
        MoveSelectorConfig::SublistSwapMoveSelector(selector) => builder.push(compile_list_leaf(
            ListLeafKind::SublistSwap,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        )?),
        MoveSelectorConfig::ListReverseMoveSelector(selector) => builder.push(compile_list_leaf(
            ListLeafKind::Reverse,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        )?),
        MoveSelectorConfig::KOptMoveSelector(selector) => builder.push(compile_list_leaf(
            ListLeafKind::KOpt,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        )?),
        MoveSelectorConfig::ListRuinMoveSelector(selector) => builder.push(compile_list_leaf(
            ListLeafKind::Ruin,
            config,
            candidate_order,
            candidate_metric.clone(),
            &selector.target,
            path,
            descriptor,
            model,
        )?),
        MoveSelectorConfig::GroupedScalarMoveSelector(grouped) => {
            compile_grouped_or_provider_selector(
                config,
                grouped,
                candidate_order,
                candidate_metric,
                path,
                model,
                &mut builder,
            )?
        }
        MoveSelectorConfig::ConflictRepairMoveSelector(_) => builder.push(compile_conflict_repair(
            config,
            candidate_order,
            candidate_metric,
            path,
            model,
        )?),
        MoveSelectorConfig::CompoundConflictRepairMoveSelector(_) => builder.push(
            compile_conflict_repair(config, candidate_order, candidate_metric, path, model)?,
        ),
        MoveSelectorConfig::LimitedNeighborhood(limit) => {
            let selector = compile_selector(
                limit.selector.as_ref(),
                default_candidate_order,
                &format!("{path}.selector"),
                descriptor,
                model,
            )?;
            let selector_root = rebase_graph(&mut builder, selector);
            builder.push(CompiledSelectorNode::Limited {
                selected_count_limit: limit.selected_count_limit,
                selector: selector_root,
            })
        }
        MoveSelectorConfig::UnionMoveSelector(union) => {
            if union.selectors.is_empty() {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::EmptyUnion,
                });
            }
            let children = union
                .selectors
                .iter()
                .enumerate()
                .map(|(index, child)| {
                    compile_selector(
                        child,
                        default_candidate_order,
                        &format!("{path}.selectors[{index}]"),
                        descriptor,
                        model,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let weighting_error = match union.weighting {
                solverforge_config::UnionWeighting::Equal if !union.weights.is_empty() => {
                    Some("equal union weighting does not accept explicit weights")
                }
                solverforge_config::UnionWeighting::Fixed
                    if union.weights.len() != children.len() =>
                {
                    Some("fixed union weight count must match selector count")
                }
                solverforge_config::UnionWeighting::Fixed if union.weights.contains(&0) => {
                    Some("fixed union weights must all be positive")
                }
                solverforge_config::UnionWeighting::CandidateCount if !union.weights.is_empty() => {
                    Some("candidate_count union weighting does not accept explicit weights")
                }
                solverforge_config::UnionWeighting::CandidateCount
                    if children.iter().any(|child| child.contains_provider_root()) =>
                {
                    Some("candidate_count union weighting requires statically countable children")
                }
                solverforge_config::UnionWeighting::Fixed
                | solverforge_config::UnionWeighting::CandidateCount
                    if !matches!(
                        union.selection_order,
                        solverforge_config::UnionSelectionOrder::Random
                            | solverforge_config::UnionSelectionOrder::StratifiedRandom
                    ) =>
                {
                    Some("weighted union selection requires random or stratified_random order")
                }
                _ => None,
            };
            if let Some(message) = weighting_error {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::LocalSearchShape {
                        message: message.to_string(),
                    },
                });
            }
            let children = children
                .into_iter()
                .map(|child| rebase_graph(&mut builder, child))
                .collect();
            builder.push(CompiledSelectorNode::Union {
                selection_order: union.selection_order,
                weighting: union.weighting,
                weights: union.weights.clone(),
                children,
            })
        }
        MoveSelectorConfig::CartesianProductMoveSelector(cartesian) => {
            if cartesian.selectors.len() != 2 {
                return Err(RuntimeCompileError {
                    path: path.to_string(),
                    kind: RuntimeCompileErrorKind::InvalidCartesianArity {
                        actual: cartesian.selectors.len(),
                    },
                });
            }
            let left = compile_selector(
                &cartesian.selectors[0],
                default_candidate_order,
                &format!("{path}.selectors[0]"),
                descriptor,
                model,
            )?;
            if left.requires_score_during_move_root() {
                return Err(RuntimeCompileError {
                    path: format!("{path}.selectors[0]"),
                    kind: RuntimeCompileErrorKind::PreviewUnsafeCartesianLeft,
                });
            }
            // Compilation only binds the right child.  The future cursor opens
            // it only after the left preview is legal, preserving deferred
            // provider invocation for nested Cartesian selectors.
            let right = compile_selector(
                &cartesian.selectors[1],
                default_candidate_order,
                &format!("{path}.selectors[1]"),
                descriptor,
                model,
            )?;
            let left_root = rebase_graph(&mut builder, left);
            let right_root = rebase_graph(&mut builder, right);
            builder.push(CompiledSelectorNode::Cartesian {
                require_hard_improvement: cartesian.require_hard_improvement,
                left: left_root,
                right: right_root,
            })
        }
    };
    Ok(builder.assemble_graph(root))
}

/* Compiles the grouped-scalar family into either its distinct assignment
node or one generic provider node. Separate function keeps the match arms
uniform. */
