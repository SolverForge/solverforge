use std::any::Any;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::{ConstructionHeuristicConfig, ConstructionHeuristicType};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::selector::EntityReference;
use crate::phase::construction::{
    BestFitForager, ConstructionHeuristicPhase, EntityPlacer, FirstFitForager, Placement,
    StrongestFitForager, WeakestFitForager,
};
use crate::scope::{ProgressCallback, SolverScope};

use super::bindings::ResolvedVariableBinding;
use super::move_types::{DescriptorChangeMove, DescriptorScalarMoveUnion};

#[cfg(test)]
use super::bindings::collect_bindings;

pub enum DescriptorConstruction<S: PlanningSolution> {
    FirstFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorScalarMoveUnion<S>,
            DescriptorEntityPlacer<S>,
            FirstFitForager<S, DescriptorScalarMoveUnion<S>>,
        >,
    ),
    BestFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorScalarMoveUnion<S>,
            DescriptorEntityPlacer<S>,
            BestFitForager<S, DescriptorScalarMoveUnion<S>>,
        >,
    ),
    WeakestFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorScalarMoveUnion<S>,
            DescriptorEntityPlacer<S>,
            WeakestFitForager<S, DescriptorScalarMoveUnion<S>>,
        >,
    ),
    StrongestFit(
        ConstructionHeuristicPhase<
            S,
            DescriptorScalarMoveUnion<S>,
            DescriptorEntityPlacer<S>,
            StrongestFitForager<S, DescriptorScalarMoveUnion<S>>,
        >,
    ),
}

impl<S: PlanningSolution> Debug for DescriptorConstruction<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FirstFit(phase) => write!(f, "DescriptorConstruction::FirstFit({phase:?})"),
            Self::BestFit(phase) => write!(f, "DescriptorConstruction::BestFit({phase:?})"),
            Self::WeakestFit(phase) => {
                write!(f, "DescriptorConstruction::WeakestFit({phase:?})")
            }
            Self::StrongestFit(phase) => {
                write!(f, "DescriptorConstruction::StrongestFit({phase:?})")
            }
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
            Self::WeakestFit(phase) => phase.solve(solver_scope),
            Self::StrongestFit(phase) => phase.solve(solver_scope),
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "DescriptorConstruction"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntityOrder {
    Canonical,
    AscendingKey,
    DescendingKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueOrder {
    Canonical,
    AscendingKey,
}

#[derive(Clone)]
pub struct DescriptorEntityPlacer<S> {
    bindings: Vec<ResolvedVariableBinding<S>>,
    solution_descriptor: SolutionDescriptor,
    entity_order: EntityOrder,
    value_order: ValueOrder,
    value_candidate_limit: Option<usize>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorEntityPlacer<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorEntityPlacer")
            .field("bindings", &self.bindings)
            .field("entity_order", &self.entity_order)
            .field("value_order", &self.value_order)
            .field("value_candidate_limit", &self.value_candidate_limit)
            .finish()
    }
}

impl<S> DescriptorEntityPlacer<S>
where
    S: PlanningSolution + 'static,
{
    fn new(
        bindings: Vec<ResolvedVariableBinding<S>>,
        solution_descriptor: SolutionDescriptor,
        entity_order: EntityOrder,
        value_order: ValueOrder,
        value_candidate_limit: Option<usize>,
    ) -> Self {
        Self {
            bindings,
            solution_descriptor,
            entity_order,
            value_order,
            value_candidate_limit,
            _phantom: PhantomData,
        }
    }

    fn entity_order_key(
        &self,
        binding: &ResolvedVariableBinding<S>,
        solution: &S,
        entity_index: usize,
    ) -> i64 {
        binding.entity_order_key(solution, entity_index).unwrap_or_else(|| {
            unreachable!(
                "validated descriptor scalar construction must provide construction_entity_order_key for {}.{}",
                binding.entity_type_name,
                binding.variable_name
            )
        })
    }

    fn value_order_key(
        &self,
        binding: &ResolvedVariableBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> i64 {
        binding
            .value_order_key(solution, entity_index, value)
            .unwrap_or_else(|| {
                unreachable!(
                    "validated descriptor scalar construction must provide construction_value_order_key for {}.{}",
                    binding.entity_type_name,
                    binding.variable_name
                )
            })
    }
}

impl<S> EntityPlacer<S, DescriptorScalarMoveUnion<S>> for DescriptorEntityPlacer<S>
where
    S: PlanningSolution + 'static,
{
    fn get_placements<D: Director<S>>(
        &self,
        score_director: &D,
    ) -> Vec<Placement<S, DescriptorScalarMoveUnion<S>>> {
        let mut placements = Vec::new();
        let solution = score_director.working_solution();
        let erased_solution = solution as &dyn Any;

        for binding in &self.bindings {
            let count = score_director
                .entity_count(binding.descriptor_index)
                .unwrap_or(0);
            let mut entity_indices: Vec<_> = (0..count).collect();
            if self.entity_order != EntityOrder::Canonical {
                entity_indices.sort_by(|left, right| {
                    let left_key = self.entity_order_key(binding, solution, *left);
                    let right_key = self.entity_order_key(binding, solution, *right);
                    match self.entity_order {
                        EntityOrder::Canonical => left.cmp(right),
                        EntityOrder::AscendingKey => left_key.cmp(&right_key).then(left.cmp(right)),
                        EntityOrder::DescendingKey => {
                            right_key.cmp(&left_key).then(left.cmp(right))
                        }
                    }
                });
            }

            for entity_index in entity_indices {
                let entity = self
                    .solution_descriptor
                    .get_entity(erased_solution, binding.descriptor_index, entity_index)
                    .expect("entity lookup failed for descriptor construction");
                if (binding.getter)(entity).is_some() {
                    continue;
                }

                let mut values: Vec<_> = binding
                    .candidate_values_for_entity_index(
                        &self.solution_descriptor,
                        erased_solution,
                        entity_index,
                        self.value_candidate_limit,
                    )
                    .into_iter()
                    .enumerate()
                    .collect();
                if self.value_order != ValueOrder::Canonical {
                    values.sort_by(|(left_order, left_value), (right_order, right_value)| {
                        let left_key =
                            self.value_order_key(binding, solution, entity_index, *left_value);
                        let right_key =
                            self.value_order_key(binding, solution, entity_index, *right_value);
                        match self.value_order {
                            ValueOrder::Canonical => left_order.cmp(right_order),
                            ValueOrder::AscendingKey => {
                                left_key.cmp(&right_key).then(left_order.cmp(right_order))
                            }
                        }
                    });
                }

                let moves = values
                    .into_iter()
                    .map(|(_, value)| {
                        let mut mov = DescriptorChangeMove::new(
                            binding.clone_binding(),
                            entity_index,
                            Some(value),
                            self.solution_descriptor.clone(),
                        );
                        if let Some(order_key) = binding.runtime_value_order_key() {
                            mov = mov.with_construction_value_order_key(order_key);
                        }
                        DescriptorScalarMoveUnion::Change(mov)
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

fn descriptor_scalar_move_strength<S>(mov: &DescriptorScalarMoveUnion<S>, solution: &S) -> i64
where
    S: PlanningSolution + 'static,
    S::Score: Score,
{
    match mov {
        DescriptorScalarMoveUnion::Change(mov) => mov.live_value_order_key(solution).unwrap_or(0),
        _ => 0,
    }
}

fn entity_order_for_heuristic(heuristic: ConstructionHeuristicType) -> EntityOrder {
    match heuristic {
        ConstructionHeuristicType::FirstFitDecreasing
        | ConstructionHeuristicType::WeakestFitDecreasing
        | ConstructionHeuristicType::StrongestFitDecreasing => EntityOrder::DescendingKey,
        ConstructionHeuristicType::AllocateEntityFromQueue => EntityOrder::AscendingKey,
        _ => EntityOrder::Canonical,
    }
}

fn value_order_for_heuristic(heuristic: ConstructionHeuristicType) -> ValueOrder {
    match heuristic {
        ConstructionHeuristicType::AllocateToValueFromQueue => ValueOrder::AscendingKey,
        _ => ValueOrder::Canonical,
    }
}

fn heuristic_requires_live_refresh(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
            | ConstructionHeuristicType::AllocateEntityFromQueue
            | ConstructionHeuristicType::AllocateToValueFromQueue
    )
}

fn build_descriptor_phase<S, Fo>(
    placer: &DescriptorEntityPlacer<S>,
    heuristic: ConstructionHeuristicType,
    forager: Fo,
) -> ConstructionHeuristicPhase<S, DescriptorScalarMoveUnion<S>, DescriptorEntityPlacer<S>, Fo>
where
    S: PlanningSolution + 'static,
    S::Score: Score,
    Fo: crate::phase::construction::ConstructionForager<S, DescriptorScalarMoveUnion<S>>,
{
    let phase = ConstructionHeuristicPhase::new(placer.clone(), forager);
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
        "descriptor scalar construction requires at least one resolved binding",
    );
    let construction_type = config
        .map(|cfg| cfg.construction_heuristic_type)
        .unwrap_or(ConstructionHeuristicType::FirstFit);
    let value_candidate_limit = config.and_then(|cfg| cfg.value_candidate_limit);
    if construction_type == ConstructionHeuristicType::CheapestInsertion {
        let unbounded = bindings
            .iter()
            .any(|binding| binding.candidate_values.is_none() && value_candidate_limit.is_none());
        assert!(
            !unbounded,
            "cheapest_insertion descriptor scalar construction requires candidate_values or value_candidate_limit",
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
        | ConstructionHeuristicType::AllocateToValueFromQueue => DescriptorConstruction::FirstFit(
            build_descriptor_phase(&placer, construction_type, FirstFitForager::new()),
        ),
        ConstructionHeuristicType::CheapestInsertion => DescriptorConstruction::BestFit(
            build_descriptor_phase(&placer, construction_type, BestFitForager::new()),
        ),
        ConstructionHeuristicType::WeakestFit | ConstructionHeuristicType::WeakestFitDecreasing => {
            DescriptorConstruction::WeakestFit(build_descriptor_phase(
                &placer,
                construction_type,
                WeakestFitForager::new(descriptor_scalar_move_strength::<S>),
            ))
        }
        ConstructionHeuristicType::StrongestFit
        | ConstructionHeuristicType::StrongestFitDecreasing => {
            DescriptorConstruction::StrongestFit(build_descriptor_phase(
                &placer,
                construction_type,
                StrongestFitForager::new(descriptor_scalar_move_strength::<S>),
            ))
        }
        ConstructionHeuristicType::ListRoundRobin
        | ConstructionHeuristicType::ListCheapestInsertion
        | ConstructionHeuristicType::ListRegretInsertion
        | ConstructionHeuristicType::ListClarkeWright
        | ConstructionHeuristicType::ListKOpt => {
            unreachable!(
                "descriptor scalar construction only handles scalar heuristics, got {:?}",
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
