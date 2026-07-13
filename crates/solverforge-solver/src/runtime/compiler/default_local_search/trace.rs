use solverforge_config::{MoveSelectorConfig, UnionSelectionOrder, VariableTargetConfig};

use crate::builder::RuntimeScalarSlotId;
use crate::stats::CandidateTracePhasePlan;

use super::{DefaultLocalSearchPlan, DefaultLocalSearchSelectorDeclaration};

impl DefaultLocalSearchPlan {
    /// The compiler can prove this declaration exactly; a later runner may
    /// publish it as the resolved local-search child without rebuilding a
    /// configuration-shaped lookalike plan.
    pub(crate) fn candidate_trace_plan(&self) -> CandidateTracePhasePlan {
        let mut attributes = vec![
            (
                "acceptor",
                self.components.acceptor.trace_label().to_string(),
            ),
            ("forager", self.components.forager.trace_label().to_string()),
            (
                "selection_order",
                selection_order_label(self.selection_order).to_string(),
            ),
            ("selector_count", self.selectors.len().to_string()),
        ];
        match self.components.acceptor {
            super::DefaultLocalSearchAcceptorPolicy::LateAcceptance { history_size }
            | super::DefaultLocalSearchAcceptorPolicy::DiversifiedLateAcceptance { history_size } => {
                attributes.push(("acceptor_history_size", history_size.to_string()))
            }
            super::DefaultLocalSearchAcceptorPolicy::SimulatedAnnealing {
                decay_rate_bits,
                random_seed,
            } => {
                attributes.push((
                    "simulated_annealing_decay_rate_bits",
                    decay_rate_bits.to_string(),
                ));
                attributes.push((
                    "simulated_annealing_random_seed",
                    random_seed.map_or_else(|| "none".to_string(), |seed| seed.to_string()),
                ));
            }
        }
        match self.components.forager {
            super::DefaultLocalSearchForagerPolicy::AcceptedCount { limit } => {
                attributes.push(("forager_accepted_count_limit", limit.to_string()));
            }
            super::DefaultLocalSearchForagerPolicy::FirstLastStepScoreImproving {
                accepted_count_limit,
            } => {
                attributes.push((
                    "forager_accepted_count_limit",
                    accepted_count_limit
                        .map_or_else(|| "none".to_string(), |limit| limit.to_string()),
                ));
            }
        }
        CandidateTracePhasePlan::known(
            "solverforge.runtime.default_local_search",
            attributes,
            self.selectors
                .iter()
                .map(DefaultLocalSearchSelectorDeclaration::candidate_trace_plan)
                .collect(),
        )
    }

    /// Returns only the selector subtree for an explicit local-search phase
    /// that omitted its selector. Acceptor, forager, and termination
    /// provenance remain owned by that explicit phase.
    pub(crate) fn candidate_trace_selector_children(&self) -> Vec<CandidateTracePhasePlan> {
        self.selectors
            .iter()
            .map(DefaultLocalSearchSelectorDeclaration::candidate_trace_plan)
            .collect()
    }
}

impl DefaultLocalSearchSelectorDeclaration {
    fn candidate_trace_plan(&self) -> CandidateTracePhasePlan {
        CandidateTracePhasePlan::known(
            "solverforge.runtime.default_local_search.selector",
            [
                (
                    "capability_policy",
                    self.capability_policy.trace_label().to_string(),
                ),
                ("config", selector_config_signature(&self.config)),
                ("family", self.family.trace_label().to_string()),
                (
                    "slots",
                    self.slots
                        .iter()
                        .map(slot_signature)
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ],
            Vec::new(),
        )
    }
}

pub(crate) fn selection_order_label(order: UnionSelectionOrder) -> &'static str {
    match order {
        UnionSelectionOrder::Sequential => "sequential",
        UnionSelectionOrder::RoundRobin => "round_robin",
        UnionSelectionOrder::RotatingRoundRobin => "rotating_round_robin",
        UnionSelectionOrder::Random => "random",
        UnionSelectionOrder::StratifiedRandom => "stratified_random",
    }
}

pub(crate) fn slot_signature(slot: &RuntimeScalarSlotId) -> String {
    format!(
        "{}:{}:{}:{}",
        slot.descriptor_index, slot.variable_index, slot.entity_class, slot.variable_name,
    )
}

pub(crate) fn selector_config_signature(config: &MoveSelectorConfig) -> String {
    match config {
        MoveSelectorConfig::ChangeMoveSelector(config) => format!(
            "change(value_candidate_limit={:?},target={})",
            config.value_candidate_limit,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::SwapMoveSelector(config) => {
            format!("swap(target={})", target_signature(&config.target))
        }
        MoveSelectorConfig::NearbyChangeMoveSelector(config) => format!(
            "nearby_change(max_nearby={},value_candidate_limit={:?},target={})",
            config.max_nearby,
            config.value_candidate_limit,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::NearbySwapMoveSelector(config) => format!(
            "nearby_swap(max_nearby={},target={})",
            config.max_nearby,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::PillarChangeMoveSelector(config) => format!(
            "pillar_change(minimum_sub_pillar_size={},maximum_sub_pillar_size={},value_candidate_limit={:?},target={})",
            config.minimum_sub_pillar_size,
            config.maximum_sub_pillar_size,
            config.value_candidate_limit,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::PillarSwapMoveSelector(config) => format!(
            "pillar_swap(minimum_sub_pillar_size={},maximum_sub_pillar_size={},target={})",
            config.minimum_sub_pillar_size,
            config.maximum_sub_pillar_size,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::RuinRecreateMoveSelector(config) => format!(
            "ruin_recreate(min_ruin_count={},max_ruin_count={},moves_per_step={:?},value_candidate_limit={:?},recreate_heuristic_type={:?},target={})",
            config.min_ruin_count,
            config.max_ruin_count,
            config.moves_per_step,
            config.value_candidate_limit,
            config.recreate_heuristic_type,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::ListChangeMoveSelector(config) => {
            format!("list_change(target={})", target_signature(&config.target))
        }
        MoveSelectorConfig::NearbyListChangeMoveSelector(config) => format!(
            "nearby_list_change(max_nearby={},target={})",
            config.max_nearby,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::ListSwapMoveSelector(config) => {
            format!("list_swap(target={})", target_signature(&config.target))
        }
        MoveSelectorConfig::ListPermuteMoveSelector(config) => format!(
            "list_permute(min_window_size={},max_window_size={},target={})",
            config.min_window_size,
            config.max_window_size,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::ListPrecedenceMoveSelector(config) => {
            format!(
                "list_precedence(target={})",
                target_signature(&config.target)
            )
        }
        MoveSelectorConfig::NearbyListSwapMoveSelector(config) => format!(
            "nearby_list_swap(max_nearby={},target={})",
            config.max_nearby,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::SublistChangeMoveSelector(config) => format!(
            "sublist_change(min_sublist_size={},max_sublist_size={},target={})",
            config.min_sublist_size,
            config.max_sublist_size,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::SublistSwapMoveSelector(config) => format!(
            "sublist_swap(min_sublist_size={},max_sublist_size={},target={})",
            config.min_sublist_size,
            config.max_sublist_size,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::ListReverseMoveSelector(config) => {
            format!("list_reverse(target={})", target_signature(&config.target))
        }
        MoveSelectorConfig::KOptMoveSelector(config) => format!(
            "k_opt(k={},min_segment_len={},max_nearby={},target={})",
            config.k,
            config.min_segment_len,
            config.max_nearby,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::ListRuinMoveSelector(config) => format!(
            "list_ruin(min_ruin_count={},max_ruin_count={},moves_per_step={:?},max_source_list_len={:?},skip_empty_destinations={},target={})",
            config.min_ruin_count,
            config.max_ruin_count,
            config.moves_per_step,
            config.max_source_list_len,
            config.skip_empty_destinations,
            target_signature(&config.target)
        ),
        MoveSelectorConfig::LimitedNeighborhood(config) => format!(
            "limited(selected_count_limit={},selector={})",
            config.selected_count_limit,
            selector_config_signature(&config.selector)
        ),
        MoveSelectorConfig::UnionMoveSelector(config) => format!(
            "union(selection_order={},weighting={},weights={:?},selectors=[{}])",
            selection_order_label(config.selection_order),
            format!("{:?}", config.weighting).to_ascii_lowercase(),
            config.weights,
            config
                .selectors
                .iter()
                .map(selector_config_signature)
                .collect::<Vec<_>>()
                .join("|")
        ),
        MoveSelectorConfig::CartesianProductMoveSelector(config) => format!(
            "cartesian(require_hard_improvement={},selectors=[{}])",
            config.require_hard_improvement,
            config
                .selectors
                .iter()
                .map(selector_config_signature)
                .collect::<Vec<_>>()
                .join("|")
        ),
        MoveSelectorConfig::ConflictRepairMoveSelector(config) => format!(
            "conflict_repair(constraints={:?},max_matches_per_step={},max_repairs_per_match={},max_moves_per_step={},require_hard_improvement={},include_soft_matches={})",
            config.constraints,
            config.max_matches_per_step,
            config.max_repairs_per_match,
            config.max_moves_per_step,
            config.require_hard_improvement,
            config.include_soft_matches
        ),
        MoveSelectorConfig::GroupedScalarMoveSelector(config) => format!(
            "grouped_scalar(group_name={},value_candidate_limit={:?},max_moves_per_step={:?},require_hard_improvement={})",
            config.group_name,
            config.value_candidate_limit,
            config.max_moves_per_step,
            config.require_hard_improvement
        ),
        MoveSelectorConfig::CompoundConflictRepairMoveSelector(config) => format!(
            "compound_conflict_repair(constraints={:?},max_matches_per_step={},max_repairs_per_match={},max_moves_per_step={},include_soft_matches={})",
            config.constraints,
            config.max_matches_per_step,
            config.max_repairs_per_match,
            config.max_moves_per_step,
            config.include_soft_matches
        ),
    }
}

fn target_signature(target: &VariableTargetConfig) -> String {
    format!(
        "{}:{}",
        target.entity_class.as_deref().unwrap_or("*"),
        target.variable_name.as_deref().unwrap_or("*")
    )
}
