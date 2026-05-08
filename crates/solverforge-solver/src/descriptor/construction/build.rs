use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;

#[cfg(test)]
use crate::descriptor::bindings::collect_bindings;
use crate::descriptor::bindings::ResolvedVariableBinding;
use crate::descriptor::move_types::DescriptorMoveUnion;
use crate::phase::construction::{
    BestFitForager, ConstructionHeuristicPhase, FirstFitForager, StrongestFitForager,
    WeakestFitForager,
};

use super::placer::{
    descriptor_move_strength, entity_order_for_heuristic, heuristic_requires_live_refresh,
    value_order_for_heuristic, DescriptorConstruction, DescriptorEntityPlacer,
};

fn build_descriptor_phase<S, Fo>(
    placer: &DescriptorEntityPlacer<S>,
    heuristic: ConstructionHeuristicType,
    construction_obligation: ConstructionObligation,
    forager: Fo,
) -> ConstructionHeuristicPhase<S, DescriptorMoveUnion<S>, DescriptorEntityPlacer<S>, Fo>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
    Fo: crate::phase::construction::ConstructionForager<S, DescriptorMoveUnion<S>>,
{
    let phase = ConstructionHeuristicPhase::new(placer.clone(), forager)
        .with_construction_obligation(construction_obligation);
    if heuristic_requires_live_refresh(heuristic) {
        phase.with_live_placement_refresh()
    } else {
        phase
    }
}

pub(crate) fn build_descriptor_construction_from_bindings<S>(
    config: Option<&ConstructionHeuristicConfig>,
    descriptor: &SolutionDescriptor,
    bindings: Vec<ResolvedVariableBinding<S>>,
) -> DescriptorConstruction<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    assert!(
        !bindings.is_empty(),
        "descriptor-driven construction requires at least one resolved binding",
    );
    let construction_type = config
        .map(|cfg| cfg.construction_heuristic_type)
        .unwrap_or(ConstructionHeuristicType::FirstFit);
    let construction_obligation = config
        .map(|cfg| cfg.construction_obligation)
        .unwrap_or_default();
    let value_candidate_limit = config.and_then(|cfg| cfg.value_candidate_limit);
    if construction_type == ConstructionHeuristicType::CheapestInsertion {
        let unbounded = bindings
            .iter()
            .any(|binding| binding.candidate_values.is_none() && value_candidate_limit.is_none());
        assert!(
            !unbounded,
            "cheapest_insertion descriptor-driven construction requires candidate_values or value_candidate_limit",
        );
    }
    let placer = DescriptorEntityPlacer::new(
        bindings,
        descriptor.clone(),
        entity_order_for_heuristic(construction_type),
        value_order_for_heuristic(construction_type),
        value_candidate_limit,
    );

    match construction_type {
        ConstructionHeuristicType::FirstFit
        | ConstructionHeuristicType::FirstFitDecreasing
        | ConstructionHeuristicType::AllocateEntityFromQueue
        | ConstructionHeuristicType::AllocateToValueFromQueue => {
            DescriptorConstruction::FirstFit(build_descriptor_phase(
                &placer,
                construction_type,
                construction_obligation,
                FirstFitForager::new(),
            ))
        }
        ConstructionHeuristicType::CheapestInsertion => {
            DescriptorConstruction::BestFit(build_descriptor_phase(
                &placer,
                construction_type,
                construction_obligation,
                BestFitForager::new(),
            ))
        }
        ConstructionHeuristicType::WeakestFit | ConstructionHeuristicType::WeakestFitDecreasing => {
            DescriptorConstruction::WeakestFit(build_descriptor_phase(
                &placer,
                construction_type,
                construction_obligation,
                WeakestFitForager::new(descriptor_move_strength::<S>),
            ))
        }
        ConstructionHeuristicType::StrongestFit
        | ConstructionHeuristicType::StrongestFitDecreasing => {
            DescriptorConstruction::StrongestFit(build_descriptor_phase(
                &placer,
                construction_type,
                construction_obligation,
                StrongestFitForager::new(descriptor_move_strength::<S>),
            ))
        }
        ConstructionHeuristicType::ListRoundRobin
        | ConstructionHeuristicType::ListCheapestInsertion
        | ConstructionHeuristicType::ListRegretInsertion
        | ConstructionHeuristicType::ListClarkeWright
        | ConstructionHeuristicType::ListKOpt => {
            unreachable!(
                "descriptor-driven construction only handles scalar heuristics, got {:?}",
                construction_type
            );
        }
    }
}

#[cfg(test)]
pub(crate) fn build_descriptor_construction<S>(
    config: Option<&ConstructionHeuristicConfig>,
    descriptor: &SolutionDescriptor,
) -> DescriptorConstruction<S>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    let bindings = collect_bindings(descriptor)
        .into_iter()
        .map(ResolvedVariableBinding::new)
        .collect();
    build_descriptor_construction_from_bindings(config, descriptor, bindings)
}
