use solverforge_config::{LocalSearchConfig, LocalSearchType, SelectionOrder};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::builder::RuntimeModel;

use super::default_local_search::DefaultLocalSearchPlan;
use super::graph::{CompiledAcceptorForagerSelector, CompiledLocalSearch};
use super::selector_tree::compile_selector;
use super::types::{RuntimeCompileError, RuntimeCompileErrorKind};

pub(super) fn compile_local_search<S, V, DM, IDM>(
    config: &LocalSearchConfig,
    path: &str,
    descriptor: &SolutionDescriptor,
    model: &RuntimeModel<S, V, DM, IDM>,
    default_local_search: Option<&DefaultLocalSearchPlan>,
) -> Result<CompiledLocalSearch<S, V, DM, IDM>, RuntimeCompileError>
where
    S: PlanningSolution + 'static,
    V: Clone,
    DM: Clone,
    IDM: Clone,
{
    match config.local_search_type {
        LocalSearchType::AcceptorForager => {
            if !config.neighborhoods.is_empty() {
                return Err(local_search_shape(
                    path,
                    "acceptor_forager local_search uses move_selector; neighborhoods are only valid with local_search_type = \"variable_neighborhood_descent\"",
                ));
            }
            let selector = match config.move_selector.as_ref() {
                Some(selector) => CompiledAcceptorForagerSelector::Explicit(compile_selector(
                    selector,
                    SelectionOrder::Random,
                    &format!("{path}.move_selector"),
                    descriptor,
                    model,
                )?),
                None => {
                    default_local_search.ok_or_else(|| {
                        local_search_shape(
                            path,
                            "acceptor_forager omitted move_selector but no immutable default selector was compiled",
                        )
                    })?;
                    CompiledAcceptorForagerSelector::OmittedDefault
                }
            };
            Ok(CompiledLocalSearch::AcceptorForager {
                config: config.clone(),
                selector,
            })
        }
        LocalSearchType::VariableNeighborhoodDescent => {
            if config.acceptor.is_some()
                || config.forager.is_some()
                || config.move_selector.is_some()
            {
                return Err(local_search_shape(
                    path,
                    "variable_neighborhood_descent local_search uses neighborhoods; acceptor, forager, and move_selector are only valid with local_search_type = \"acceptor_forager\"",
                ));
            }
            if config.neighborhoods.is_empty() {
                return Err(local_search_shape(
                    path,
                    "variable_neighborhood_descent local_search requires at least one neighborhood selector",
                ));
            }
            let neighborhoods = config
                .neighborhoods
                .iter()
                .enumerate()
                .map(|(index, selector)| {
                    compile_selector(
                        selector,
                        SelectionOrder::Original,
                        &format!("{path}.neighborhoods[{index}]"),
                        descriptor,
                        model,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(CompiledLocalSearch::VariableNeighborhoodDescent {
                config: config.clone(),
                neighborhoods,
            })
        }
    }
}
pub(super) fn local_search_shape(path: &str, message: impl Into<String>) -> RuntimeCompileError {
    RuntimeCompileError {
        path: path.to_string(),
        kind: RuntimeCompileErrorKind::LocalSearchShape {
            message: message.into(),
        },
    }
}
