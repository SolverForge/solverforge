use std::any::Any;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::{MoveSelectorConfig, PhaseConfig, SolverConfig};
use solverforge_core::domain::{
    SolutionDescriptor, UsizeEntityValueProvider, UsizeGetter, UsizeSetter, ValueRangeType,
};
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::{ConstraintSet, Director, ScoreDirector, SolvableSolution};
use tracing::info;

use crate::builder::{AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder};
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::decorator::VecUnionSelector;
use crate::heuristic::selector::typed_move_selector::MoveSelector;
use crate::phase::localsearch::{
    AcceptedCountForager, LocalSearchPhase, SimulatedAnnealingAcceptor,
};
use crate::problem_spec::ProblemSpec;
use crate::run::AnyTermination;
use crate::scope::{ProgressCallback, SolverScope};
use crate::solver::{SolveResult, Solver};

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

type DescriptorLocalSearch<S> = LocalSearchPhase<
    S,
    DescriptorEitherMove<S>,
    VecUnionSelector<S, DescriptorEitherMove<S>, DescriptorLeafSelector<S>>,
    AnyAcceptor<S>,
    AnyForager<S>,
>;

#[derive(Debug, Default)]
struct SeedBestSolutionPhase;

impl<S, D, ProgressCb> crate::phase::Phase<S, D, ProgressCb> for SeedBestSolutionPhase
where
    S: solverforge_core::domain::PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        let score = solver_scope.calculate_score();
        let solution = solver_scope.score_director().clone_working_solution();
        solver_scope.set_best_solution(solution, score);
    }

    fn phase_type_name(&self) -> &'static str {
        "SeedBestSolution"
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

fn find_binding(bindings: &[VariableBinding], entity_class: Option<&str>) -> Vec<VariableBinding> {
    bindings
        .iter()
        .filter(|binding| entity_class.is_none_or(|name| name == binding.entity_type_name))
        .cloned()
        .collect()
}

fn build_move_selector<S>(
    config: Option<&MoveSelectorConfig>,
    descriptor: &SolutionDescriptor,
) -> VecUnionSelector<S, DescriptorEitherMove<S>, DescriptorLeafSelector<S>>
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
                for binding in find_binding(bindings, change.entity_class.as_deref()) {
                    leaves.push(DescriptorLeafSelector::Change(
                        DescriptorChangeMoveSelector::new(binding, descriptor.clone()),
                    ));
                }
            }
            MoveSelectorConfig::SwapMoveSelector(swap) => {
                for binding in find_binding(bindings, swap.entity_class.as_deref()) {
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
            _ => {
                for binding in bindings.iter().cloned() {
                    leaves.push(DescriptorLeafSelector::Change(
                        DescriptorChangeMoveSelector::new(binding.clone(), descriptor.clone()),
                    ));
                    leaves.push(DescriptorLeafSelector::Swap(
                        DescriptorSwapMoveSelector::new(binding, descriptor.clone()),
                    ));
                }
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

    VecUnionSelector::new(leaves)
}

pub struct DescriptorStandardSpec;

impl<S, C> ProblemSpec<S, C> for DescriptorStandardSpec
where
    S: solverforge_core::domain::PlanningSolution + SolvableSolution + 'static,
    S::Score: Score + ParseableScore,
    C: ConstraintSet<S, S::Score>,
{
    fn is_trivial(&self, solution: &S) -> bool {
        let descriptor = S::descriptor();
        descriptor
            .total_entity_count(solution as &dyn Any)
            .unwrap_or(0)
            == 0
    }

    fn default_time_limit_secs(&self) -> u64 {
        30
    }

    fn log_scale(&self, solution: &S) {
        let descriptor = S::descriptor();
        info!(
            event = "solve_start",
            entity_count = descriptor
                .total_entity_count(solution as &dyn Any)
                .unwrap_or(0),
            variable_count = collect_bindings(&descriptor).len(),
        );
    }

    fn build_and_solve(
        self,
        director: ScoreDirector<S, C>,
        config: &SolverConfig,
        time_limit: Duration,
        termination: AnyTermination<S, ScoreDirector<S, C>>,
        terminate: Option<&AtomicBool>,
        callback: impl ProgressCallback<S>,
    ) -> SolveResult<S> {
        let solution_descriptor = director.solution_descriptor().clone();
        let ls_config = config.phases.iter().find_map(|phase| {
            if let PhaseConfig::LocalSearch(local_search) = phase {
                Some(local_search)
            } else {
                None
            }
        });

        let acceptor = ls_config
            .and_then(|ls| ls.acceptor.as_ref())
            .map(AcceptorBuilder::build::<S>)
            .unwrap_or_else(|| {
                AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::default())
            });

        let forager = ls_config
            .and_then(|ls| ls.forager.as_ref())
            .map(|_| ForagerBuilder::build::<S>(ls_config.and_then(|ls| ls.forager.as_ref())))
            .unwrap_or_else(|| AnyForager::AcceptedCount(AcceptedCountForager::new(1)));

        let move_selector = build_move_selector(
            ls_config.and_then(|ls| ls.move_selector.as_ref()),
            &solution_descriptor,
        );
        let local_search = DescriptorLocalSearch::new(move_selector, acceptor, forager, None);
        let solver = Solver::new(((), SeedBestSolutionPhase, local_search))
            .with_termination(termination)
            .with_time_limit(time_limit)
            .with_progress_callback(callback);

        if let Some(flag) = terminate {
            solver.with_terminate(flag).solve(director)
        } else {
            solver.solve(director)
        }
    }
}
