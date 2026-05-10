pub type Selector<S, V, DM, IDM> =
    VecUnionSelector<S, NeighborhoodMove<S, V>, Neighborhood<S, V, DM, IDM>>;

pub type LocalSearch<S, V, DM, IDM> = LocalSearchPhase<
    S,
    NeighborhoodMove<S, V>,
    Selector<S, V, DM, IDM>,
    AnyAcceptor<S>,
    AnyForager<S>,
>;

pub struct LocalSearchStrategy<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    inner: LocalSearchStrategyInner<S, V, DM, IDM>,
}

#[allow(clippy::large_enum_variant)] // Inline storage keeps local-search phases zero-erasure.
enum LocalSearchStrategyInner<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    AcceptorForager(LocalSearch<S, V, DM, IDM>),
    VariableNeighborhoodDescent(VndPhase<S, NeighborhoodMove<S, V>, Neighborhood<S, V, DM, IDM>>),
}

impl<S, V, DM, IDM> LocalSearchStrategy<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn acceptor_forager(phase: LocalSearch<S, V, DM, IDM>) -> Self {
        Self {
            inner: LocalSearchStrategyInner::AcceptorForager(phase),
        }
    }

    fn variable_neighborhood_descent(
        phase: VndPhase<S, NeighborhoodMove<S, V>, Neighborhood<S, V, DM, IDM>>,
    ) -> Self {
        Self {
            inner: LocalSearchStrategyInner::VariableNeighborhoodDescent(phase),
        }
    }
}

impl<S, V, DM, IDM> Debug for LocalSearchStrategy<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            LocalSearchStrategyInner::AcceptorForager(phase) => f
                .debug_tuple("LocalSearchStrategy::AcceptorForager")
                .field(phase)
                .finish(),
            LocalSearchStrategyInner::VariableNeighborhoodDescent(phase) => f
                .debug_tuple("LocalSearchStrategy::VariableNeighborhoodDescent")
                .field(phase)
                .finish(),
        }
    }
}

impl<S, V, DM, IDM, D, ProgressCb> Phase<S, D, ProgressCb>
    for LocalSearchStrategy<S, V, DM, IDM>
where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + Debug + 'static,
    D: solverforge_scoring::Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        match &mut self.inner {
            LocalSearchStrategyInner::AcceptorForager(phase) => phase.solve(solver_scope),
            LocalSearchStrategyInner::VariableNeighborhoodDescent(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "LocalSearch"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectorFamily {
    Scalar,
    List,
    Mixed,
    Unsupported,
}

fn selector_family(config: &MoveSelectorConfig) -> SelectorFamily {
    match config {
        MoveSelectorConfig::ChangeMoveSelector(_)
        | MoveSelectorConfig::SwapMoveSelector(_)
        | MoveSelectorConfig::NearbyChangeMoveSelector(_)
        | MoveSelectorConfig::NearbySwapMoveSelector(_)
        | MoveSelectorConfig::PillarChangeMoveSelector(_)
        | MoveSelectorConfig::PillarSwapMoveSelector(_)
        | MoveSelectorConfig::RuinRecreateMoveSelector(_)
        | MoveSelectorConfig::GroupedScalarMoveSelector(_)
        | MoveSelectorConfig::ConflictRepairMoveSelector(_)
        | MoveSelectorConfig::CompoundConflictRepairMoveSelector(_) => SelectorFamily::Scalar,
        MoveSelectorConfig::ListChangeMoveSelector(_)
        | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
        | MoveSelectorConfig::ListSwapMoveSelector(_)
        | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
        | MoveSelectorConfig::SublistChangeMoveSelector(_)
        | MoveSelectorConfig::SublistSwapMoveSelector(_)
        | MoveSelectorConfig::ListReverseMoveSelector(_)
        | MoveSelectorConfig::KOptMoveSelector(_)
        | MoveSelectorConfig::ListRuinMoveSelector(_) => SelectorFamily::List,
        MoveSelectorConfig::LimitedNeighborhood(limit) => selector_family(limit.selector.as_ref()),
        MoveSelectorConfig::UnionMoveSelector(union) => {
            let mut family = None;
            for child in &union.selectors {
                let child_family = selector_family(child);
                if child_family == SelectorFamily::Unsupported {
                    return SelectorFamily::Unsupported;
                }
                family = Some(match family {
                    None => child_family,
                    Some(current) if current == child_family => current,
                    Some(_) => SelectorFamily::Mixed,
                });
                if family == Some(SelectorFamily::Mixed) {
                    return SelectorFamily::Mixed;
                }
            }
            family.unwrap_or(SelectorFamily::Mixed)
        }
        MoveSelectorConfig::CartesianProductMoveSelector(_) => SelectorFamily::Unsupported,
    }
}

fn selector_family_name(config: Option<&MoveSelectorConfig>) -> &'static str {
    match config.map(selector_family) {
        None => "default",
        Some(SelectorFamily::Scalar) => "scalar",
        Some(SelectorFamily::List) => "list",
        Some(SelectorFamily::Mixed) => "mixed",
        Some(SelectorFamily::Unsupported) => "unsupported",
    }
}

fn selector_requires_score_during_move(config: &MoveSelectorConfig) -> bool {
    match config {
        MoveSelectorConfig::RuinRecreateMoveSelector(_)
        | MoveSelectorConfig::ListRuinMoveSelector(_) => true,
        MoveSelectorConfig::LimitedNeighborhood(limit) => {
            selector_requires_score_during_move(limit.selector.as_ref())
        }
        MoveSelectorConfig::UnionMoveSelector(union) => union
            .selectors
            .iter()
            .any(selector_requires_score_during_move),
        MoveSelectorConfig::CartesianProductMoveSelector(_) => true,
        _ => false,
    }
}

fn assert_cartesian_left_preview_safe(config: &MoveSelectorConfig) {
    assert!(
        !selector_requires_score_during_move(config),
        "cartesian_product left child cannot contain ruin_recreate_move_selector or list_ruin_move_selector because preview directors do not calculate scores",
    );
}

fn push_scalar_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
    out: &mut Vec<NeighborhoodLeaf<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    let scalar_variables: Vec<_> = model.scalar_variables().copied().collect();
    if scalar_variables.is_empty() {
        return;
    }
    if let Some(config) = config {
        if let Some(variable) = assignment_owned_scalar_target(config, model) {
            panic!(
                "scalar move selector targets assignment-owned scalar variable {}.{}; use the owning grouped scalar assignment selector instead",
                variable.entity_type_name,
                variable.variable_name
            );
        }
    }
    let selector = build_scalar_flat_selector(config, &scalar_variables, random_seed);
    out.extend(
        selector
            .into_selectors()
            .into_iter()
            .map(NeighborhoodLeaf::Scalar),
    );
}

fn assignment_owned_scalar_target<S, V, DM, IDM>(
    config: &MoveSelectorConfig,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Option<crate::builder::ScalarVariableSlot<S>> {
    match config {
        MoveSelectorConfig::ChangeMoveSelector(config) => {
            target_assignment_owner(&config.target, model)
        }
        MoveSelectorConfig::SwapMoveSelector(config) => target_assignment_owner(&config.target, model),
        MoveSelectorConfig::NearbyChangeMoveSelector(config) => {
            target_assignment_owner(&config.target, model)
        }
        MoveSelectorConfig::NearbySwapMoveSelector(config) => {
            target_assignment_owner(&config.target, model)
        }
        MoveSelectorConfig::PillarChangeMoveSelector(config) => {
            target_assignment_owner(&config.target, model)
        }
        MoveSelectorConfig::PillarSwapMoveSelector(config) => {
            target_assignment_owner(&config.target, model)
        }
        MoveSelectorConfig::RuinRecreateMoveSelector(config) => {
            target_assignment_owner(&config.target, model)
        }
        MoveSelectorConfig::LimitedNeighborhood(limit) => {
            assignment_owned_scalar_target(limit.selector.as_ref(), model)
        }
        MoveSelectorConfig::UnionMoveSelector(union) => union
            .selectors
            .iter()
            .find_map(|selector| assignment_owned_scalar_target(selector, model)),
        _ => None,
    }
}

fn target_assignment_owner<S, V, DM, IDM>(
    target: &solverforge_config::VariableTargetConfig,
    model: &RuntimeModel<S, V, DM, IDM>,
) -> Option<crate::builder::ScalarVariableSlot<S>> {
    model.scalar_variables().copied().find(|variable| {
        variable.matches_target(
            target.entity_class.as_deref(),
            target.variable_name.as_deref(),
        ) && model.assignment_group_covers_scalar_variable(variable)
    })
}

fn push_list_selector<S, V, DM, IDM>(
    config: Option<&MoveSelectorConfig>,
    model: &RuntimeModel<S, V, DM, IDM>,
    random_seed: Option<u64>,
    out: &mut Vec<NeighborhoodLeaf<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    for variable in model.list_variables() {
        let selector = ListMoveSelectorBuilder::build_flat(config, variable, random_seed);
        out.extend(
            selector
                .into_selectors()
                .into_iter()
                .map(NeighborhoodLeaf::List),
        );
    }
}

fn push_conflict_repair_selector<S, V, DM, IDM>(
    config: &solverforge_config::ConflictRepairMoveSelectorConfig,
    model: &RuntimeModel<S, V, DM, IDM>,
    out: &mut Vec<NeighborhoodLeaf<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    if config.constraints.is_empty() {
        panic!("conflict_repair_move_selector requires at least one constraint");
    }
    let scalar_variables = model.scalar_variables().copied().collect::<Vec<_>>();
    let repairs = model
        .conflict_repairs()
        .iter()
        .copied()
        .filter(|repair| {
            config
                .constraints
                .iter()
                .any(|constraint| constraint == repair.constraint_name())
        })
        .collect::<Vec<_>>();
    if repairs.is_empty() {
        panic!(
            "conflict_repair_move_selector configured for {:?}, but no matching providers were registered",
            config.constraints
        );
    }
    out.push(NeighborhoodLeaf::ConflictRepair(ConflictRepairSelector::new(
        config.clone(),
        scalar_variables,
        repairs,
    )));
}

fn push_compound_conflict_repair_selector<S, V, DM, IDM>(
    config: &solverforge_config::CompoundConflictRepairMoveSelectorConfig,
    model: &RuntimeModel<S, V, DM, IDM>,
    out: &mut Vec<NeighborhoodLeaf<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    if config.constraints.is_empty() {
        panic!("compound_conflict_repair_move_selector requires at least one constraint");
    }
    let scalar_variables = model.scalar_variables().copied().collect::<Vec<_>>();
    let repairs = model
        .conflict_repairs()
        .iter()
        .copied()
        .filter(|repair| {
            config
                .constraints
                .iter()
                .any(|constraint| constraint == repair.constraint_name())
        })
        .collect::<Vec<_>>();
    if repairs.is_empty() {
        panic!(
            "compound_conflict_repair_move_selector configured for {:?}, but no matching providers were registered",
            config.constraints
        );
    }
    out.push(NeighborhoodLeaf::ConflictRepair(
        ConflictRepairSelector::new_compound(config.clone(), scalar_variables, repairs),
    ));
}

fn push_grouped_scalar_selector<S, V, DM, IDM>(
    config: &solverforge_config::GroupedScalarMoveSelectorConfig,
    model: &RuntimeModel<S, V, DM, IDM>,
    out: &mut Vec<NeighborhoodLeaf<S, V, DM, IDM>>,
) where
    S: PlanningSolution + 'static,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: CrossEntityDistanceMeter<S> + Clone + 'static,
    IDM: CrossEntityDistanceMeter<S> + Clone + 'static,
{
    let Some(group) = model
        .scalar_groups()
        .iter()
        .find(|group| group.group_name == config.group_name)
        .cloned()
    else {
        panic!(
            "grouped_scalar_move_selector configured for `{}`, but no matching scalar group was registered",
            config.group_name
        );
    };
    out.push(NeighborhoodLeaf::GroupedScalar(GroupedScalarSelector::new(
        group,
        config.value_candidate_limit,
        config.max_moves_per_step,
        config.require_hard_improvement,
    )));
}
