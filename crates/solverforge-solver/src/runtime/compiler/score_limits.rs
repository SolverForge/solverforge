use solverforge_config::{PhaseConfig, SolverConfig, TerminationConfig};
use solverforge_core::score::ParseableScore;

use super::types::{RuntimeCompileError, RuntimeCompileErrorKind};

/// Rejects every `best_score_limit` in `config` that does not parse as the
/// model's score, naming its key path, so no solve starts with a limit it
/// would otherwise ignore.
pub(super) fn validate_best_score_limits<Sc: ParseableScore>(
    config: &SolverConfig,
) -> Result<(), RuntimeCompileError> {
    validate_termination::<Sc>(config.termination.as_ref(), "termination")?;
    validate_phases::<Sc>(&config.phases, "phases")
}

fn validate_phases<Sc: ParseableScore>(
    phases: &[PhaseConfig],
    path: &str,
) -> Result<(), RuntimeCompileError> {
    for (index, phase) in phases.iter().enumerate() {
        let path = format!("{path}[{index}]");
        match phase {
            PhaseConfig::ConstructionHeuristic(config) => validate_termination::<Sc>(
                config.termination.as_ref(),
                &format!("{path}.construction.termination"),
            )?,
            PhaseConfig::LocalSearch(config) => validate_termination::<Sc>(
                config.termination.as_ref(),
                &format!("{path}.local_search.termination"),
            )?,
            PhaseConfig::PartitionedSearch(config) => {
                let path = format!("{path}.partitioned_search");
                validate_termination::<Sc>(
                    config.termination.as_ref(),
                    &format!("{path}.termination"),
                )?;
                validate_phases::<Sc>(&config.child_phases, &format!("{path}.child_phases"))?;
            }
            PhaseConfig::Custom(_) => {}
        }
    }
    Ok(())
}

fn validate_termination<Sc: ParseableScore>(
    config: Option<&TerminationConfig>,
    path: &str,
) -> Result<(), RuntimeCompileError> {
    let Some(score) = config.and_then(|config| config.best_score_limit.as_deref()) else {
        return Ok(());
    };
    match Sc::parse(score) {
        Ok(_) => Ok(()),
        Err(error) => Err(RuntimeCompileError {
            path: format!("{path}.best_score_limit"),
            kind: RuntimeCompileErrorKind::InvalidValue {
                message: format!("`{score}` is not a valid score for this model: {error}"),
            },
        }),
    }
}
