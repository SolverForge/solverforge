use super::*;

/// Parsed solver termination policy shared by runtime phase assembly and the
/// top-level termination builder.
///
/// `TerminationConfig` historically chooses the first configured score/work
/// criterion in this order: best score, step count, unimproved steps,
/// unimproved time. A configured time limit is paired with that criterion, or
/// is the policy itself when no other criterion is present. Keeping that
/// precedence here prevents phase assembly from treating an empty or
/// unparsable configuration as a finite solver boundary.
#[derive(Clone, Copy)]
pub(crate) struct ConfiguredTermination<Sc> {
    pub(super) time_limit: Option<Duration>,
    criterion: Option<ConfiguredTerminationCriterion<Sc>>,
}

#[derive(Clone, Copy)]
enum ConfiguredTerminationCriterion<Sc> {
    BestScore(Sc),
    StepCount(u64),
    UnimprovedStepCount(u64),
    UnimprovedTime(Duration),
}

impl<Sc> ConfiguredTermination<Sc> {
    pub(crate) fn has_effective_limit(&self) -> bool {
        self.time_limit.is_some() || self.criterion.is_some()
    }
}

/// Returns a termination's `best_score_limit` as the model's score.
///
/// `compile_runtime_graph` rejects a limit that does not parse before any
/// solve starts, so every reader after compilation sees either no limit or a
/// valid one.
pub(crate) fn configured_best_score_limit<Sc: ParseableScore>(
    config: &TerminationConfig,
) -> Option<Sc> {
    config.best_score_limit.as_deref().map(|score| {
        Sc::parse(score).unwrap_or_else(|error| {
            panic!("best_score_limit `{score}` reached a solve unvalidated: {error}")
        })
    })
}

pub(crate) fn parse_configured_termination<S>(
    config: Option<&TerminationConfig>,
) -> ConfiguredTermination<S::Score>
where
    S: PlanningSolution,
    S::Score: ParseableScore,
{
    let time_limit = config.and_then(TerminationConfig::time_limit);
    let criterion = config.and_then(|config| {
        configured_best_score_limit::<S::Score>(config)
            .map(ConfiguredTerminationCriterion::BestScore)
            .or_else(|| {
                config
                    .step_count_limit
                    .map(ConfiguredTerminationCriterion::StepCount)
            })
            .or_else(|| {
                config
                    .unimproved_step_count_limit
                    .map(ConfiguredTerminationCriterion::UnimprovedStepCount)
            })
            .or_else(|| {
                config
                    .unimproved_time_limit()
                    .map(ConfiguredTerminationCriterion::UnimprovedTime)
            })
    });
    ConfiguredTermination {
        time_limit,
        criterion,
    }
}

/// Builds a termination from config, returning both the termination and the time limit.
pub(crate) fn build_director_termination<S, D>(
    config: &SolverConfig,
    default_secs: u64,
) -> (AnyTermination<S, D>, Option<Duration>)
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
    D: Director<S>,
{
    let ConfiguredTermination {
        time_limit: configured_time_limit,
        criterion,
    } = parse_configured_termination::<S>(config.termination.as_ref());
    let fallback_time_limit = Duration::from_secs(default_secs);

    let (termination, effective_time_limit) = match criterion {
        Some(ConfiguredTerminationCriterion::BestScore(target)) => {
            let effective_time_limit = configured_time_limit.unwrap_or(fallback_time_limit);
            let time = TimeTermination::new(effective_time_limit);
            (
                AnyTermination::WithBestScore(OrTermination::new((
                    time,
                    BestScoreTermination::new(target),
                ))),
                Some(effective_time_limit),
            )
        }
        Some(ConfiguredTerminationCriterion::StepCount(step_limit)) => {
            let effective_time_limit = configured_time_limit.unwrap_or(fallback_time_limit);
            let time = TimeTermination::new(effective_time_limit);
            (
                AnyTermination::WithStepCount(OrTermination::new((
                    time,
                    StepCountTermination::new(step_limit),
                ))),
                Some(effective_time_limit),
            )
        }
        Some(ConfiguredTerminationCriterion::UnimprovedStepCount(unimproved_step_limit)) => {
            let effective_time_limit = configured_time_limit.unwrap_or(fallback_time_limit);
            let time = TimeTermination::new(effective_time_limit);
            (
                AnyTermination::WithUnimprovedStep(OrTermination::new((
                    time,
                    UnimprovedStepCountTermination::<S>::new(unimproved_step_limit),
                ))),
                Some(effective_time_limit),
            )
        }
        Some(ConfiguredTerminationCriterion::UnimprovedTime(unimproved_time)) => {
            let effective_time_limit = configured_time_limit.unwrap_or(fallback_time_limit);
            let time = TimeTermination::new(effective_time_limit);
            (
                AnyTermination::WithUnimprovedTime(OrTermination::new((
                    time,
                    UnimprovedTimeTermination::<S>::new(unimproved_time),
                ))),
                Some(effective_time_limit),
            )
        }
        None => configured_time_limit.map_or_else(
            || (AnyTermination::None(NoTermination), None),
            |limit| {
                let time = TimeTermination::new(limit);
                (
                    AnyTermination::Default(OrTermination::new((time,))),
                    Some(limit),
                )
            },
        ),
    };

    (termination, effective_time_limit)
}

/// Records the termination policy the configured runtime actually installed.
///
/// This deliberately derives its time guard from `build_termination`'s
/// returned effective limit rather than from the input TOML.  In particular,
/// a score/work criterion without an explicit time limit gets the configured
/// entrypoint's fallback guard, and that injected guard is material to both
/// bounded-work and fixed-budget comparisons.
pub(crate) fn configured_execution_policy<S>(
    config: &SolverConfig,
    default_secs: u64,
    effective_time_limit: Option<Duration>,
) -> CandidateTraceExecutionPolicy
where
    S: PlanningSolution,
    S::Score: ParseableScore + std::fmt::Display,
{
    let configured = parse_configured_termination::<S>(config.termination.as_ref());
    let configured_time_limit = configured.time_limit;
    let criterion = configured.criterion;
    let fallback_time_limit = Duration::from_secs(default_secs);

    let time_limit_source = match (configured_time_limit, effective_time_limit) {
        (Some(_), Some(_)) => "configured",
        (None, Some(_)) if criterion.is_some() => "configured_entrypoint_fallback",
        (None, Some(_)) => "internal",
        (_, None) => "not_installed",
    };
    let mut attributes = vec![
        ("entrypoint".to_string(), "configured_runtime".to_string()),
        (
            "configured_time_limit_ns".to_string(),
            configured_time_limit.map_or_else(|| "none".to_string(), duration_nanos),
        ),
        (
            "configured_entrypoint_default_time_limit_ns".to_string(),
            duration_nanos(fallback_time_limit),
        ),
        (
            "effective_time_limit_ns".to_string(),
            effective_time_limit.map_or_else(|| "none".to_string(), duration_nanos),
        ),
        (
            "time_limit_source".to_string(),
            time_limit_source.to_string(),
        ),
    ];

    match criterion {
        Some(ConfiguredTerminationCriterion::BestScore(target)) => {
            attributes.push(("criterion".to_string(), "best_score".to_string()));
            attributes.push(("criterion_target".to_string(), target.to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "time_or_best_score".to_string(),
            ));
        }
        Some(ConfiguredTerminationCriterion::StepCount(limit)) => {
            attributes.push(("criterion".to_string(), "step_count".to_string()));
            attributes.push(("criterion_target".to_string(), limit.to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "time_or_step_count".to_string(),
            ));
        }
        Some(ConfiguredTerminationCriterion::UnimprovedStepCount(limit)) => {
            attributes.push(("criterion".to_string(), "unimproved_step_count".to_string()));
            attributes.push(("criterion_target".to_string(), limit.to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "time_or_unimproved_step_count".to_string(),
            ));
        }
        Some(ConfiguredTerminationCriterion::UnimprovedTime(limit)) => {
            attributes.push(("criterion".to_string(), "unimproved_time".to_string()));
            attributes.push(("criterion_target_ns".to_string(), duration_nanos(limit)));
            attributes.push((
                "termination_composition".to_string(),
                "time_or_unimproved_time".to_string(),
            ));
        }
        None if effective_time_limit.is_some() => {
            attributes.push(("criterion".to_string(), "none".to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "time_only".to_string(),
            ));
        }
        None => {
            attributes.push(("criterion".to_string(), "none".to_string()));
            attributes.push((
                "termination_composition".to_string(),
                "unbounded".to_string(),
            ));
        }
    }

    CandidateTraceExecutionPolicy::known("solverforge.execution_policy", attributes)
}

fn duration_nanos(duration: Duration) -> String {
    duration.as_nanos().to_string()
}
