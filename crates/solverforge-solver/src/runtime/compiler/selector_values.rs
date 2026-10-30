//! Domain checks for the numeric move-selector and construction-phase values.
//!
//! Each rule follows what the consuming cursor or phase can enumerate: a
//! value that would leave the neighborhood or the construction phase without
//! a single candidate, or that the cursor would silently replace, is
//! rejected with its key path before any phase runs.

use solverforge_config::{
    ConstructionHeuristicConfig, KOptMoveSelectorConfig, ListPermuteMoveConfig, MoveSelectorConfig,
    PhaseConfig, SolverConfig,
};

use crate::heuristic::r#move::MAX_LIST_PERMUTE_WINDOW_SIZE;

use super::types::{RuntimeCompileError, RuntimeCompileErrorKind};

/// Largest k the k-opt reconnection kernel enumerates.
const MAX_K_OPT_K: usize = 5;

/// The only k the list k-opt construction kernel implements.
const LIST_K_OPT_CONSTRUCTION_K: usize = 2;

/// Rejects every out-of-domain numeric move-selector or construction value in
/// `config`, including nested selectors and partitioned child phases.
pub(super) fn validate_selector_values(config: &SolverConfig) -> Result<(), RuntimeCompileError> {
    validate_phases(&config.phases, "phases")
}

fn validate_phases(phases: &[PhaseConfig], path: &str) -> Result<(), RuntimeCompileError> {
    for (index, phase) in phases.iter().enumerate() {
        let path = format!("{path}[{index}]");
        match phase {
            PhaseConfig::ConstructionHeuristic(config) => {
                validate_construction(config, &format!("{path}.construction"))?;
            }
            PhaseConfig::LocalSearch(config) => {
                let path = format!("{path}.local_search");
                if let Some(selector) = config.move_selector.as_ref() {
                    validate_selector(selector, &format!("{path}.move_selector"))?;
                }
                for (index, selector) in config.neighborhoods.iter().enumerate() {
                    validate_selector(selector, &format!("{path}.neighborhoods[{index}]"))?;
                }
            }
            PhaseConfig::PartitionedSearch(config) => validate_phases(
                &config.child_phases,
                &format!("{path}.partitioned_search.child_phases"),
            )?,
            PhaseConfig::Custom(_) => {}
        }
    }
    Ok(())
}

fn validate_construction(
    config: &ConstructionHeuristicConfig,
    path: &str,
) -> Result<(), RuntimeCompileError> {
    if config.k != LIST_K_OPT_CONSTRUCTION_K {
        return Err(invalid(
            path,
            "k",
            format!(
                "must be {LIST_K_OPT_CONSTRUCTION_K}: list k-opt construction implements only 2-opt, got {}",
                config.k
            ),
        ));
    }
    positive_limit(path, "value_candidate_limit", config.value_candidate_limit)?;
    positive_limit(path, "group_candidate_limit", config.group_candidate_limit)
}

fn validate_selector(config: &MoveSelectorConfig, path: &str) -> Result<(), RuntimeCompileError> {
    match config {
        MoveSelectorConfig::ChangeMoveSelector(config) => {
            positive_limit(path, "value_candidate_limit", config.value_candidate_limit)
        }
        MoveSelectorConfig::NearbyChangeMoveSelector(config) => {
            positive(path, "max_nearby", config.max_nearby)?;
            positive_limit(path, "value_candidate_limit", config.value_candidate_limit)
        }
        MoveSelectorConfig::NearbySwapMoveSelector(config) => {
            positive(path, "max_nearby", config.max_nearby)
        }
        MoveSelectorConfig::PillarChangeMoveSelector(config) => {
            sub_pillar_sizes(
                path,
                config.minimum_sub_pillar_size,
                config.maximum_sub_pillar_size,
            )?;
            positive_limit(path, "value_candidate_limit", config.value_candidate_limit)
        }
        MoveSelectorConfig::PillarSwapMoveSelector(config) => sub_pillar_sizes(
            path,
            config.minimum_sub_pillar_size,
            config.maximum_sub_pillar_size,
        ),
        MoveSelectorConfig::RuinRecreateMoveSelector(config) => {
            ruin_counts(path, config.min_ruin_count, config.max_ruin_count)?;
            positive_limit(path, "moves_per_step", config.moves_per_step)?;
            positive_limit(path, "value_candidate_limit", config.value_candidate_limit)
        }
        MoveSelectorConfig::GroupedScalarMoveSelector(config) => {
            positive_limit(path, "value_candidate_limit", config.value_candidate_limit)?;
            positive_limit(path, "max_moves_per_step", config.max_moves_per_step)
        }
        MoveSelectorConfig::NearbyListChangeMoveSelector(config) => {
            positive(path, "max_nearby", config.max_nearby)
        }
        MoveSelectorConfig::NearbyListSwapMoveSelector(config) => {
            positive(path, "max_nearby", config.max_nearby)
        }
        MoveSelectorConfig::ListPermuteMoveSelector(config) => permute_windows(path, config),
        MoveSelectorConfig::SublistChangeMoveSelector(config) => {
            sublist_sizes(path, config.min_sublist_size, config.max_sublist_size)
        }
        MoveSelectorConfig::SublistSwapMoveSelector(config) => {
            sublist_sizes(path, config.min_sublist_size, config.max_sublist_size)
        }
        MoveSelectorConfig::KOptMoveSelector(config) => k_opt(path, config),
        MoveSelectorConfig::ListRuinMoveSelector(config) => {
            ruin_counts(path, config.min_ruin_count, config.max_ruin_count)?;
            positive_limit(path, "moves_per_step", config.moves_per_step)?;
            positive_limit(path, "max_source_list_len", config.max_source_list_len)
        }
        MoveSelectorConfig::LimitedNeighborhood(config) => {
            positive(path, "selected_count_limit", config.selected_count_limit)?;
            validate_selector(&config.selector, &format!("{path}.selector"))
        }
        MoveSelectorConfig::UnionMoveSelector(config) => validate_children(&config.selectors, path),
        MoveSelectorConfig::CartesianProductMoveSelector(config) => {
            validate_children(&config.selectors, path)
        }
        MoveSelectorConfig::ConflictRepairMoveSelector(config) => repair_limits(
            path,
            config.max_matches_per_step,
            config.max_repairs_per_match,
            config.max_moves_per_step,
        ),
        MoveSelectorConfig::CompoundConflictRepairMoveSelector(config) => repair_limits(
            path,
            config.max_matches_per_step,
            config.max_repairs_per_match,
            config.max_moves_per_step,
        ),
        MoveSelectorConfig::SwapMoveSelector(_)
        | MoveSelectorConfig::ListChangeMoveSelector(_)
        | MoveSelectorConfig::ListSwapMoveSelector(_)
        | MoveSelectorConfig::ListPrecedenceMoveSelector(_)
        | MoveSelectorConfig::ListReverseMoveSelector(_) => Ok(()),
    }
}

fn validate_children(
    selectors: &[MoveSelectorConfig],
    path: &str,
) -> Result<(), RuntimeCompileError> {
    for (index, selector) in selectors.iter().enumerate() {
        validate_selector(selector, &format!("{path}.selectors[{index}]"))?;
    }
    Ok(())
}

/// `0/0` keeps its documented meaning (full pillars only); otherwise both
/// sizes bound the enumerated sub-pillars and must form a range.
fn sub_pillar_sizes(path: &str, minimum: usize, maximum: usize) -> Result<(), RuntimeCompileError> {
    if minimum == 0 && maximum == 0 {
        return Ok(());
    }
    if minimum == 0 {
        return Err(invalid(
            path,
            "minimum_sub_pillar_size",
            format!(
                "must be at least 1 when maximum_sub_pillar_size is {maximum}; set both to 0 for full pillars only"
            ),
        ));
    }
    if maximum == 0 {
        return Err(invalid(
            path,
            "maximum_sub_pillar_size",
            format!(
                "must be at least 1 when minimum_sub_pillar_size is {minimum}; set both to 0 for full pillars only"
            ),
        ));
    }
    ordered(
        path,
        ("minimum_sub_pillar_size", minimum),
        ("maximum_sub_pillar_size", maximum),
    )
}

fn sublist_sizes(path: &str, minimum: usize, maximum: usize) -> Result<(), RuntimeCompileError> {
    positive(path, "min_sublist_size", minimum)?;
    ordered(
        path,
        ("min_sublist_size", minimum),
        ("max_sublist_size", maximum),
    )
}

fn ruin_counts(path: &str, minimum: usize, maximum: usize) -> Result<(), RuntimeCompileError> {
    positive(path, "min_ruin_count", minimum)?;
    ordered(
        path,
        ("min_ruin_count", minimum),
        ("max_ruin_count", maximum),
    )
}

fn permute_windows(path: &str, config: &ListPermuteMoveConfig) -> Result<(), RuntimeCompileError> {
    if config.min_window_size < 2 {
        return Err(invalid(
            path,
            "min_window_size",
            format!(
                "must be at least 2 (a window permutes two or more elements), got {}",
                config.min_window_size
            ),
        ));
    }
    if config.max_window_size > MAX_LIST_PERMUTE_WINDOW_SIZE {
        return Err(invalid(
            path,
            "max_window_size",
            format!(
                "must be at most {MAX_LIST_PERMUTE_WINDOW_SIZE}, got {}",
                config.max_window_size
            ),
        ));
    }
    ordered(
        path,
        ("min_window_size", config.min_window_size),
        ("max_window_size", config.max_window_size),
    )
}

fn k_opt(path: &str, config: &KOptMoveSelectorConfig) -> Result<(), RuntimeCompileError> {
    if !(2..=MAX_K_OPT_K).contains(&config.k) {
        return Err(invalid(
            path,
            "k",
            format!("must be between 2 and {MAX_K_OPT_K}, got {}", config.k),
        ));
    }
    positive(path, "min_segment_len", config.min_segment_len)
}

fn repair_limits(
    path: &str,
    max_matches_per_step: usize,
    max_repairs_per_match: usize,
    max_moves_per_step: usize,
) -> Result<(), RuntimeCompileError> {
    positive(path, "max_matches_per_step", max_matches_per_step)?;
    positive(path, "max_repairs_per_match", max_repairs_per_match)?;
    positive(path, "max_moves_per_step", max_moves_per_step)
}

fn ordered(
    path: &str,
    (minimum_field, minimum): (&str, usize),
    (maximum_field, maximum): (&str, usize),
) -> Result<(), RuntimeCompileError> {
    if minimum > maximum {
        return Err(invalid(
            path,
            minimum_field,
            format!("must not exceed {maximum_field} ({maximum}), got {minimum}"),
        ));
    }
    Ok(())
}

fn positive_limit(
    path: &str,
    field: &str,
    value: Option<usize>,
) -> Result<(), RuntimeCompileError> {
    match value {
        Some(value) => positive(path, field, value),
        None => Ok(()),
    }
}

fn positive(path: &str, field: &str, value: usize) -> Result<(), RuntimeCompileError> {
    if value == 0 {
        return Err(invalid(
            path,
            field,
            "must be at least 1; 0 leaves no candidate to select".to_string(),
        ));
    }
    Ok(())
}

fn invalid(path: &str, field: &str, message: String) -> RuntimeCompileError {
    RuntimeCompileError {
        path: format!("{path}.{field}"),
        kind: RuntimeCompileErrorKind::InvalidValue { message },
    }
}
