use solverforge_config::{PhaseConfig, SolverConfig};
use solverforge_core::domain::PlanningSolution;

use crate::builder::{RuntimeExtensionPolicy, RuntimeExtensionRegistry};

use super::construction::compile_construction;
use super::default_local_search::DefaultLocalSearchPlan;
use super::defaults::compile_default_runtime_bindings;
use super::graph::{CompiledRuntimeExtension, CompiledRuntimeGraph, CompiledRuntimePhase};
use super::input::RuntimeGraphInput;
use super::local_search::compile_local_search;
use super::types::{RuntimeCompileError, RuntimeCompileErrorKind, RuntimeExtensionKind};

/// Compiles one recursive graph for typed and dynamic runtime slots.
///
/// This is intentionally side-effect free: no solution import, host callback,
/// selector cursor, score calculation, random draw, or trace work occurs here.
pub(crate) fn compile_runtime_graph<S, V, DM, IDM, E>(
    config: &SolverConfig,
    input: RuntimeGraphInput<S, V, DM, IDM, E>,
) -> Result<CompiledRuntimeGraph<S, V, DM, IDM, E>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
    E: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    let (context, extensions) = input.into_parts();
    if context.seed() != config.random_seed {
        return Err(RuntimeCompileError {
            path: "runtime_extension_context.random_seed".to_string(),
            kind: RuntimeCompileErrorKind::ContextSeedMismatch {
                config_seed: config.random_seed,
                context_seed: context.seed(),
            },
        });
    }
    let needs_omitted_local_search = config.phases.is_empty()
        || config.phases.iter().any(|phase| {
            matches!(
                phase,
                PhaseConfig::LocalSearch(local_search)
                    if local_search.local_search_type == solverforge_config::LocalSearchType::AcceptorForager
                        && local_search.move_selector.is_none()
            )
        });
    let default_bindings = compile_default_runtime_bindings(
        context.descriptor(),
        context.model(),
        needs_omitted_local_search,
        config.random_seed,
    )?;
    let phases = if config.phases.is_empty() {
        // Defaults are explicitly retained as a graph node. They are
        // solution-state-sensitive (unassigned elements, owner feasibility),
        // so the future instantiator resolves their child sequence once per
        // solve from this compiled model rather than inventing a static plan.
        vec![CompiledRuntimePhase::DefaultRuntime]
    } else {
        config
            .phases
            .iter()
            .enumerate()
            .map(|(index, phase)| {
                compile_phase(
                    phase,
                    &format!("phases[{index}]"),
                    context.descriptor(),
                    context.model(),
                    &extensions,
                    default_bindings.local_search_plan.as_ref(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok(CompiledRuntimeGraph {
        context,
        extensions,
        config: config.clone(),
        default_bindings,
        phases,
    })
}

pub(super) fn compile_phase<S, V, DM, IDM, E>(
    phase: &PhaseConfig,
    path: &str,
    descriptor: &solverforge_core::domain::SolutionDescriptor,
    model: &crate::builder::RuntimeModel<S, V, DM, IDM>,
    extensions: &E,
    default_local_search: Option<&DefaultLocalSearchPlan>,
) -> Result<CompiledRuntimePhase<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
    E: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    match phase {
        PhaseConfig::ConstructionHeuristic(config) => Ok(CompiledRuntimePhase::Construction(
            compile_construction(config, &format!("{path}.construction"), descriptor, model)?,
        )),
        PhaseConfig::LocalSearch(config) => {
            Ok(CompiledRuntimePhase::LocalSearch(compile_local_search(
                config,
                &format!("{path}.local_search"),
                descriptor,
                model,
                default_local_search,
            )?))
        }
        PhaseConfig::PartitionedSearch(config) => {
            compile_partitioned_extension(path, config, extensions)
        }
        PhaseConfig::Custom(config) => compile_custom_extension(path, &config.name, extensions),
    }
}

fn compile_custom_extension<S, V, DM, IDM, E>(
    path: &str,
    name: &str,
    extensions: &E,
) -> Result<CompiledRuntimePhase<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution,
    E: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    if name.is_empty() {
        return Err(extension_error(
            path,
            RuntimeCompileErrorKind::MissingCustomExtensionName,
        ));
    }
    if extensions.policy() == RuntimeExtensionPolicy::Dynamic {
        return Err(extension_error(
            path,
            RuntimeCompileErrorKind::UnsupportedDynamicExtension {
                extension: RuntimeExtensionKind::Custom,
            },
        ));
    }
    if !extensions.contains_custom(name) {
        return Err(extension_error(
            path,
            RuntimeCompileErrorKind::UnregisteredTypedCustomExtension {
                name: name.to_string(),
            },
        ));
    }
    Ok(CompiledRuntimePhase::Extension(
        CompiledRuntimeExtension::Custom {
            name: name.to_string(),
        },
    ))
}

fn compile_partitioned_extension<S, V, DM, IDM, E>(
    path: &str,
    config: &solverforge_config::PartitionedSearchConfig,
    extensions: &E,
) -> Result<CompiledRuntimePhase<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution,
    E: RuntimeExtensionRegistry<S, V, DM, IDM>,
{
    let Some(name) = config
        .partitioner
        .as_deref()
        .filter(|name| !name.is_empty())
    else {
        return Err(extension_error(
            path,
            RuntimeCompileErrorKind::MissingPartitionerName,
        ));
    };
    if extensions.policy() == RuntimeExtensionPolicy::Dynamic {
        return Err(extension_error(
            path,
            RuntimeCompileErrorKind::UnsupportedDynamicExtension {
                extension: RuntimeExtensionKind::Partitioned,
            },
        ));
    }
    if !extensions.contains_partitioned(name) {
        return Err(extension_error(
            path,
            RuntimeCompileErrorKind::UnregisteredTypedPartitioner {
                name: name.to_string(),
            },
        ));
    }
    Ok(CompiledRuntimePhase::Extension(
        CompiledRuntimeExtension::Partitioned {
            name: name.to_string(),
            config: config.clone(),
        },
    ))
}

fn extension_error(path: &str, kind: RuntimeCompileErrorKind) -> RuntimeCompileError {
    RuntimeCompileError {
        path: path.to_string(),
        kind,
    }
}
