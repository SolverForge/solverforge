use std::any::Any;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_scoring::Director;

use crate::heuristic::selector::EntityReference;
use crate::phase::construction::{
    BestFitForager, ConstructionHeuristicPhase, EntityPlacer, FirstFitForager, Placement,
};
use crate::scope::{ProgressCallback, SolverScope};

use super::bindings::{collect_bindings, find_binding, VariableBinding};
use super::move_types::{DescriptorChangeMove, DescriptorEitherMove};

pub enum DescriptorConstruction<S: PlanningSolution> {
    FirstFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorEitherMove<S>,
            DescriptorEntityPlacer<S>,
            FirstFitForager<S, DescriptorEitherMove<S>>,
        >,
    ),
    BestFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorEitherMove<S>,
            DescriptorEntityPlacer<S>,
            BestFitForager<S, DescriptorEitherMove<S>>,
        >,
    ),
}

impl<S: PlanningSolution> Debug for DescriptorConstruction<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FirstFit(phase) => write!(f, "DescriptorConstruction::FirstFit({phase:?})"),
            Self::BestFit(phase) => write!(f, "DescriptorConstruction::BestFit({phase:?})"),
        }
    }
}

impl<S, D, ProgressCb> crate::phase::Phase<S, D, ProgressCb> for DescriptorConstruction<S>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        match self {
            Self::FirstFit(phase) => phase.solve(solver_scope),
            Self::BestFit(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "DescriptorConstruction"
    }
}

#[derive(Clone)]
pub struct DescriptorEntityPlacer<S> {
    bindings: Vec<VariableBinding>,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorEntityPlacer<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorEntityPlacer")
            .field("bindings", &self.bindings)
            .finish()
    }
}

impl<S> DescriptorEntityPlacer<S> {
    fn new(bindings: Vec<VariableBinding>, solution_descriptor: SolutionDescriptor) -> Self {
        Self {
            bindings,
            solution_descriptor,
            _phantom: PhantomData,
        }
    }
}

impl<S> EntityPlacer<S, DescriptorEitherMove<S>> for DescriptorEntityPlacer<S>
where
    S: PlanningSolution + 'static,
{
    fn get_placements<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> Vec<Placement<S, DescriptorEitherMove<S>>> {
        let mut placements = Vec::new();

        for binding in &self.bindings {
            let count = score_director
                .entity_count(binding.descriptor_index)
                .unwrap_or(0);

            for entity_index in 0..count {
                let entity = self
                    .solution_descriptor
                    .get_entity(
                        score_director.working_solution() as &dyn Any,
                        binding.descriptor_index,
                        entity_index,
                    )
                    .expect("entity lookup failed for descriptor construction");
                let current_value = (binding.getter)(entity);
                if current_value.is_some() {
                    continue;
                }

                let moves = binding
                    .values_for_entity(
                        &self.solution_descriptor,
                        score_director.working_solution() as &dyn Any,
                        entity,
                    )
                    .into_iter()
                    .map(|value| {
                        DescriptorEitherMove::Change(DescriptorChangeMove::new(
                            binding.clone(),
                            entity_index,
                            Some(value),
                            self.solution_descriptor.clone(),
                        ))
                    })
                    .collect::<Vec<_>>();

                if moves.is_empty() {
                    continue;
                }

                placements.push(
                    Placement::new(
                        EntityReference::new(binding.descriptor_index, entity_index),
                        moves,
                    )
                    .with_slot_id(binding.slot_id(entity_index))
                    .with_keep_current_legal(binding.allows_unassigned),
                );
            }
        }

        placements
    }
}

pub fn build_descriptor_construction<S>(
    config: Option<&ConstructionHeuristicConfig>,
    descriptor: &SolutionDescriptor,
) -> DescriptorConstruction<S>
where
    S: PlanningSolution + 'static,
{
    let bindings = config
        .map(|cfg| {
            let matched = find_binding(
                &collect_bindings(descriptor),
                cfg.target.entity_class.as_deref(),
                cfg.target.variable_name.as_deref(),
            );
            assert!(
                !matched.is_empty(),
                "construction heuristic matched no standard planning variables for entity_class={:?} variable_name={:?}",
                cfg.target.entity_class,
                cfg.target.variable_name
            );
            matched
        })
        .unwrap_or_else(|| collect_bindings(descriptor));
    let placer = DescriptorEntityPlacer::new(bindings, descriptor.clone());
    let construction_type = config
        .map(|cfg| cfg.construction_heuristic_type)
        .unwrap_or(ConstructionHeuristicType::FirstFit);

    match construction_type {
        ConstructionHeuristicType::FirstFit => DescriptorConstruction::FirstFit(
            ConstructionHeuristicPhase::new(placer, FirstFitForager::new()),
        ),
        ConstructionHeuristicType::CheapestInsertion => DescriptorConstruction::BestFit(
            ConstructionHeuristicPhase::new(placer, BestFitForager::new()),
        ),
        ConstructionHeuristicType::FirstFitDecreasing
        | ConstructionHeuristicType::WeakestFit
        | ConstructionHeuristicType::WeakestFitDecreasing
        | ConstructionHeuristicType::StrongestFit
        | ConstructionHeuristicType::StrongestFitDecreasing
        | ConstructionHeuristicType::AllocateEntityFromQueue
        | ConstructionHeuristicType::AllocateToValueFromQueue
        | ConstructionHeuristicType::ListRoundRobin
        | ConstructionHeuristicType::ListCheapestInsertion
        | ConstructionHeuristicType::ListRegretInsertion
        | ConstructionHeuristicType::ListClarkeWright
        | ConstructionHeuristicType::ListKOpt => {
            panic!(
                "descriptor standard construction does not support {:?}",
                construction_type
            );
        }
    }
}
