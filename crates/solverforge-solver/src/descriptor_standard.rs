use std::any::Any;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, MoveSelectorConfig,
};
use solverforge_core::domain::{
    SolutionDescriptor, UsizeEntityValueProvider, UsizeGetter, UsizeSetter, ValueRangeType,
};
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::typed_move_selector::MoveSelector;
use crate::heuristic::selector::EntityReference;
use crate::phase::construction::{
    BestFitForager, ConstructionHeuristicPhase, EntityPlacer, FirstFitForager, Placement,
};
use crate::scope::{ProgressCallback, SolverScope};

#[derive(Clone)]
struct VariableBinding {
    descriptor_index: usize,
    entity_type_name: &'static str,
    variable_name: &'static str,
    getter: UsizeGetter,
    setter: UsizeSetter,
    provider: Option<UsizeEntityValueProvider>,
    range_type: ValueRangeType,
}

impl Debug for VariableBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VariableBinding")
            .field("descriptor_index", &self.descriptor_index)
            .field("entity_type_name", &self.entity_type_name)
            .field("variable_name", &self.variable_name)
            .field("range_type", &self.range_type)
            .finish()
    }
}

impl VariableBinding {
    fn values_for_entity(&self, entity: &dyn Any) -> Vec<usize> {
        match (&self.provider, &self.range_type) {
            (Some(provider), _) => provider(entity),
            (_, ValueRangeType::CountableRange { from, to }) => {
                let start = *from;
                let end = *to;
                (start..end)
                    .filter_map(|value| usize::try_from(value).ok())
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct DescriptorChangeMove<S> {
    binding: VariableBinding,
    entity_index: usize,
    to_value: Option<usize>,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorChangeMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorChangeMove")
            .field("descriptor_index", &self.binding.descriptor_index)
            .field("entity_index", &self.entity_index)
            .field("variable_name", &self.binding.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S: 'static> DescriptorChangeMove<S> {
    fn new(
        binding: VariableBinding,
        entity_index: usize,
        to_value: Option<usize>,
        solution_descriptor: SolutionDescriptor,
    ) -> Self {
        Self {
            binding,
            entity_index,
            to_value,
            solution_descriptor,
            _phantom: PhantomData,
        }
    }

    fn current_value(&self, solution: &S) -> Option<usize> {
        let entity = self
            .solution_descriptor
            .get_entity(
                solution as &dyn Any,
                self.binding.descriptor_index,
                self.entity_index,
            )
            .expect("entity lookup failed for descriptor change move");
        (self.binding.getter)(entity)
    }
}

impl<S> Move<S> for DescriptorChangeMove<S>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        self.current_value(score_director.working_solution()) != self.to_value
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        let old_value = self.current_value(score_director.working_solution());
        score_director.before_variable_changed(self.binding.descriptor_index, self.entity_index);
        let entity = self
            .solution_descriptor
            .get_entity_mut(
                score_director.working_solution_mut() as &mut dyn Any,
                self.binding.descriptor_index,
                self.entity_index,
            )
            .expect("entity lookup failed for descriptor change move");
        (self.binding.setter)(entity, self.to_value);
        score_director.after_variable_changed(self.binding.descriptor_index, self.entity_index);

        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        let entity_index = self.entity_index;
        score_director.register_undo(Box::new(move |solution: &mut S| {
            let entity = descriptor
                .get_entity_mut(
                    solution as &mut dyn Any,
                    binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for descriptor change undo");
            (binding.setter)(entity, old_value);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.binding.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.binding.variable_name
    }
}

#[derive(Clone)]
pub struct DescriptorSwapMove<S> {
    binding: VariableBinding,
    left_entity_index: usize,
    right_entity_index: usize,
    indices: [usize; 2],
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorSwapMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorSwapMove")
            .field("descriptor_index", &self.binding.descriptor_index)
            .field("left_entity_index", &self.left_entity_index)
            .field("right_entity_index", &self.right_entity_index)
            .field("variable_name", &self.binding.variable_name)
            .finish()
    }
}

impl<S: 'static> DescriptorSwapMove<S> {
    fn new(
        binding: VariableBinding,
        left_entity_index: usize,
        right_entity_index: usize,
        solution_descriptor: SolutionDescriptor,
    ) -> Self {
        Self {
            binding,
            left_entity_index,
            right_entity_index,
            indices: [left_entity_index, right_entity_index],
            solution_descriptor,
            _phantom: PhantomData,
        }
    }

    fn current_value(&self, solution: &S, entity_index: usize) -> Option<usize> {
        let entity = self
            .solution_descriptor
            .get_entity(
                solution as &dyn Any,
                self.binding.descriptor_index,
                entity_index,
            )
            .expect("entity lookup failed for descriptor swap move");
        (self.binding.getter)(entity)
    }
}

impl<S> Move<S> for DescriptorSwapMove<S>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        self.left_entity_index != self.right_entity_index
            && self.current_value(score_director.working_solution(), self.left_entity_index)
                != self.current_value(score_director.working_solution(), self.right_entity_index)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        let left_value =
            self.current_value(score_director.working_solution(), self.left_entity_index);
        let right_value =
            self.current_value(score_director.working_solution(), self.right_entity_index);

        score_director
            .before_variable_changed(self.binding.descriptor_index, self.left_entity_index);
        score_director
            .before_variable_changed(self.binding.descriptor_index, self.right_entity_index);

        let left_entity = self
            .solution_descriptor
            .get_entity_mut(
                score_director.working_solution_mut() as &mut dyn Any,
                self.binding.descriptor_index,
                self.left_entity_index,
            )
            .expect("entity lookup failed for descriptor swap move");
        (self.binding.setter)(left_entity, right_value);

        let right_entity = self
            .solution_descriptor
            .get_entity_mut(
                score_director.working_solution_mut() as &mut dyn Any,
                self.binding.descriptor_index,
                self.right_entity_index,
            )
            .expect("entity lookup failed for descriptor swap move");
        (self.binding.setter)(right_entity, left_value);

        score_director
            .after_variable_changed(self.binding.descriptor_index, self.left_entity_index);
        score_director
            .after_variable_changed(self.binding.descriptor_index, self.right_entity_index);

        let descriptor = self.solution_descriptor.clone();
        let binding = self.binding.clone();
        let left_entity_index = self.left_entity_index;
        let right_entity_index = self.right_entity_index;
        score_director.register_undo(Box::new(move |solution: &mut S| {
            let left_entity = descriptor
                .get_entity_mut(
                    solution as &mut dyn Any,
                    binding.descriptor_index,
                    left_entity_index,
                )
                .expect("entity lookup failed for descriptor swap undo");
            (binding.setter)(left_entity, left_value);
            let right_entity = descriptor
                .get_entity_mut(
                    solution as &mut dyn Any,
                    binding.descriptor_index,
                    right_entity_index,
                )
                .expect("entity lookup failed for descriptor swap undo");
            (binding.setter)(right_entity, right_value);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.binding.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.indices
    }

    fn variable_name(&self) -> &str {
        self.binding.variable_name
    }
}

#[derive(Clone)]
pub enum DescriptorEitherMove<S> {
    Change(DescriptorChangeMove<S>),
    Swap(DescriptorSwapMove<S>),
}

impl<S> Debug for DescriptorEitherMove<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(m) => m.fmt(f),
            Self::Swap(m) => m.fmt(f),
        }
    }
}

impl<S> Move<S> for DescriptorEitherMove<S>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        match self {
            Self::Change(m) => m.is_doable(score_director),
            Self::Swap(m) => m.is_doable(score_director),
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        match self {
            Self::Change(m) => m.do_move(score_director),
            Self::Swap(m) => m.do_move(score_director),
        }
    }

    fn descriptor_index(&self) -> usize {
        match self {
            Self::Change(m) => m.descriptor_index(),
            Self::Swap(m) => m.descriptor_index(),
        }
    }

    fn entity_indices(&self) -> &[usize] {
        match self {
            Self::Change(m) => m.entity_indices(),
            Self::Swap(m) => m.entity_indices(),
        }
    }

    fn variable_name(&self) -> &str {
        match self {
            Self::Change(m) => m.variable_name(),
            Self::Swap(m) => m.variable_name(),
        }
    }
}

#[derive(Clone)]
pub struct DescriptorChangeMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorChangeMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorChangeMoveSelector")
            .field("binding", &self.binding)
            .finish()
    }
}

impl<S> DescriptorChangeMoveSelector<S> {
    fn new(binding: VariableBinding, solution_descriptor: SolutionDescriptor) -> Self {
        Self {
            binding,
            solution_descriptor,
            _phantom: PhantomData,
        }
    }
}

impl<S> MoveSelector<S, DescriptorEitherMove<S>> for DescriptorChangeMoveSelector<S>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = DescriptorEitherMove<S>> + 'a {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let mut moves = Vec::new();
        for entity_index in 0..count {
            let entity = self
                .solution_descriptor
                .get_entity(
                    score_director.working_solution() as &dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for change selector");
            for value in self.binding.values_for_entity(entity) {
                moves.push(DescriptorEitherMove::Change(DescriptorChangeMove::new(
                    self.binding.clone(),
                    entity_index,
                    Some(value),
                    self.solution_descriptor.clone(),
                )));
            }
        }
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let mut total = 0;
        for entity_index in 0..count {
            let entity = self
                .solution_descriptor
                .get_entity(
                    score_director.working_solution() as &dyn Any,
                    self.binding.descriptor_index,
                    entity_index,
                )
                .expect("entity lookup failed for change selector");
            total += self.binding.values_for_entity(entity).len();
        }
        total
    }
}

#[derive(Clone)]
pub struct DescriptorSwapMoveSelector<S> {
    binding: VariableBinding,
    solution_descriptor: SolutionDescriptor,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for DescriptorSwapMoveSelector<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DescriptorSwapMoveSelector")
            .field("binding", &self.binding)
            .finish()
    }
}

impl<S> DescriptorSwapMoveSelector<S> {
    fn new(binding: VariableBinding, solution_descriptor: SolutionDescriptor) -> Self {
        Self {
            binding,
            solution_descriptor,
            _phantom: PhantomData,
        }
    }
}

impl<S> MoveSelector<S, DescriptorEitherMove<S>> for DescriptorSwapMoveSelector<S>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = DescriptorEitherMove<S>> + 'a {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        let mut moves = Vec::new();
        for left_entity_index in 0..count {
            for right_entity_index in (left_entity_index + 1)..count {
                moves.push(DescriptorEitherMove::Swap(DescriptorSwapMove::new(
                    self.binding.clone(),
                    left_entity_index,
                    right_entity_index,
                    self.solution_descriptor.clone(),
                )));
            }
        }
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let count = score_director
            .entity_count(self.binding.descriptor_index)
            .unwrap_or(0);
        count.saturating_mul(count.saturating_sub(1)) / 2
    }
}

#[derive(Clone)]
pub enum DescriptorLeafSelector<S> {
    Change(DescriptorChangeMoveSelector<S>),
    Swap(DescriptorSwapMoveSelector<S>),
}

impl<S> Debug for DescriptorLeafSelector<S>
where
    S: solverforge_core::domain::PlanningSolution,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Change(selector) => selector.fmt(f),
            Self::Swap(selector) => selector.fmt(f),
        }
    }
}

impl<S> MoveSelector<S, DescriptorEitherMove<S>> for DescriptorLeafSelector<S>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    fn iter_moves<'a, D: Director<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = DescriptorEitherMove<S>> + 'a {
        let moves: Vec<_> = match self {
            Self::Change(selector) => selector.iter_moves(score_director).collect(),
            Self::Swap(selector) => selector.iter_moves(score_director).collect(),
        };
        moves.into_iter()
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        match self {
            Self::Change(selector) => selector.size(score_director),
            Self::Swap(selector) => selector.size(score_director),
        }
    }
}

pub enum DescriptorConstruction<S: solverforge_core::domain::PlanningSolution> {
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

impl<S: solverforge_core::domain::PlanningSolution> Debug for DescriptorConstruction<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FirstFit(phase) => write!(f, "DescriptorConstruction::FirstFit({phase:?})"),
            Self::BestFit(phase) => write!(f, "DescriptorConstruction::BestFit({phase:?})"),
        }
    }
}

impl<S, D, ProgressCb> crate::phase::Phase<S, D, ProgressCb> for DescriptorConstruction<S>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
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
    S: solverforge_core::domain::PlanningSolution + 'static,
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
                    .values_for_entity(entity)
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

                placements.push(Placement::new(
                    EntityReference::new(binding.descriptor_index, entity_index),
                    moves,
                ));
            }
        }

        placements
    }
}

fn collect_bindings(descriptor: &SolutionDescriptor) -> Vec<VariableBinding> {
    let mut bindings = Vec::new();
    for (descriptor_index, entity_descriptor) in descriptor.entity_descriptors.iter().enumerate() {
        for variable in entity_descriptor.genuine_variable_descriptors() {
            let Some(getter) = variable.usize_getter else {
                continue;
            };
            let Some(setter) = variable.usize_setter else {
                continue;
            };
            bindings.push(VariableBinding {
                descriptor_index,
                entity_type_name: entity_descriptor.type_name,
                variable_name: variable.name,
                getter,
                setter,
                provider: variable.entity_value_provider,
                range_type: variable.value_range_type.clone(),
            });
        }
    }
    bindings
}

fn find_binding(
    bindings: &[VariableBinding],
    entity_class: Option<&str>,
    variable_name: Option<&str>,
) -> Vec<VariableBinding> {
    bindings
        .iter()
        .filter(|binding| entity_class.is_none_or(|name| name == binding.entity_type_name))
        .filter(|binding| variable_name.is_none_or(|name| name == binding.variable_name))
        .cloned()
        .collect()
}

pub fn descriptor_has_bindings(descriptor: &SolutionDescriptor) -> bool {
    !collect_bindings(descriptor).is_empty()
}

pub fn standard_work_remaining<S>(
    descriptor: &SolutionDescriptor,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
    solution: &S,
) -> bool
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    let bindings = find_binding(&collect_bindings(descriptor), entity_class, variable_name);
    for binding in bindings {
        let Some(entity_count) = descriptor
            .entity_descriptors
            .get(binding.descriptor_index)
            .and_then(|entity| entity.entity_count(solution as &dyn Any))
        else {
            continue;
        };
        for entity_index in 0..entity_count {
            let entity = descriptor
                .get_entity(solution as &dyn Any, binding.descriptor_index, entity_index)
                .expect("entity lookup failed while checking standard work");
            if (binding.getter)(entity).is_none() && !binding.values_for_entity(entity).is_empty() {
                return true;
            }
        }
    }
    false
}

pub fn standard_target_matches(
    descriptor: &SolutionDescriptor,
    entity_class: Option<&str>,
    variable_name: Option<&str>,
) -> bool {
    !find_binding(&collect_bindings(descriptor), entity_class, variable_name).is_empty()
}

fn collect_descriptor_leaf_selectors<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
) -> Vec<DescriptorLeafSelector<S>>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    let bindings = collect_bindings(descriptor);
    let mut leaves = Vec::new();

    fn collect<S>(
        cfg: &MoveSelectorConfig,
        descriptor: &SolutionDescriptor,
        bindings: &[VariableBinding],
        leaves: &mut Vec<DescriptorLeafSelector<S>>,
    ) where
        S: solverforge_core::domain::PlanningSolution + 'static,
    {
        match cfg {
            MoveSelectorConfig::ChangeMoveSelector(change) => {
                let matched = find_binding(
                    bindings,
                    change.target.entity_class.as_deref(),
                    change.target.variable_name.as_deref(),
                );
                assert!(
                    !matched.is_empty(),
                    "change_move selector matched no standard planning variables for entity_class={:?} variable_name={:?}",
                    change.target.entity_class,
                    change.target.variable_name
                );
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::Change(
                        DescriptorChangeMoveSelector::new(binding, descriptor.clone()),
                    ));
                }
            }
            MoveSelectorConfig::SwapMoveSelector(swap) => {
                let matched = find_binding(
                    bindings,
                    swap.target.entity_class.as_deref(),
                    swap.target.variable_name.as_deref(),
                );
                assert!(
                    !matched.is_empty(),
                    "swap_move selector matched no standard planning variables for entity_class={:?} variable_name={:?}",
                    swap.target.entity_class,
                    swap.target.variable_name
                );
                for binding in matched {
                    leaves.push(DescriptorLeafSelector::Swap(
                        DescriptorSwapMoveSelector::new(binding, descriptor.clone()),
                    ));
                }
            }
            MoveSelectorConfig::UnionMoveSelector(union) => {
                for child in &union.selectors {
                    collect(child, descriptor, bindings, leaves);
                }
            }
            MoveSelectorConfig::ListChangeMoveSelector(_)
            | MoveSelectorConfig::NearbyListChangeMoveSelector(_)
            | MoveSelectorConfig::ListSwapMoveSelector(_)
            | MoveSelectorConfig::NearbyListSwapMoveSelector(_)
            | MoveSelectorConfig::SubListChangeMoveSelector(_)
            | MoveSelectorConfig::SubListSwapMoveSelector(_)
            | MoveSelectorConfig::ListReverseMoveSelector(_)
            | MoveSelectorConfig::KOptMoveSelector(_)
            | MoveSelectorConfig::ListRuinMoveSelector(_) => {
                panic!("list move selector configured against a standard-variable stock context");
            }
            MoveSelectorConfig::CartesianProductMoveSelector(_) => {
                panic!("cartesian_product move selectors are not supported in stock solving");
            }
        }
    }

    match config {
        Some(cfg) => collect(cfg, descriptor, &bindings, &mut leaves),
        None => {
            for binding in bindings {
                leaves.push(DescriptorLeafSelector::Change(
                    DescriptorChangeMoveSelector::new(binding.clone(), descriptor.clone()),
                ));
                leaves.push(DescriptorLeafSelector::Swap(
                    DescriptorSwapMoveSelector::new(binding, descriptor.clone()),
                ));
            }
        }
    }

    assert!(
        !leaves.is_empty(),
        "stock move selector configuration produced no standard neighborhoods"
    );

    leaves
}

pub fn build_descriptor_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
) -> VecUnionSelector<S, DescriptorEitherMove<S>, DescriptorLeafSelector<S>>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
{
    VecUnionSelector::new(collect_descriptor_leaf_selectors(config, descriptor))
}

pub fn build_descriptor_construction<S>(
    config: Option<&ConstructionHeuristicConfig>,
    descriptor: &SolutionDescriptor,
) -> DescriptorConstruction<S>
where
    S: solverforge_core::domain::PlanningSolution + 'static,
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
