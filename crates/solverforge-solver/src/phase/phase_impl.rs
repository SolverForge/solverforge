//! Monomorphic phase enum for config-driven phase sequencing.
//!
//! This enables runtime configuration of solver phases without type erasure.
//! All phases are concrete types - NO Box<dyn Phase>.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, LocalSearchConfig, PhaseConfig,
    SolverConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::ScoreDirector;
use tracing::info;

use crate::heuristic::r#move::MoveImpl;
use crate::heuristic::selector::MoveSelectorImpl;
use crate::manager::ListConstructionPhaseBuilder;
use crate::operations::VariableOperations;
use crate::scope::SolverScope;

use super::construction::forager_impl::ConstructionForagerImpl;
use super::construction::ConstructionHeuristicPhase;
use super::localsearch::forager_impl::ForagerImpl;
use super::localsearch::{AcceptorImpl, LocalSearchPhase};
use super::Phase;

// ============================================================================
// ListPhaseImpl - Monomorphic phase enum for list-variable solutions
// ============================================================================

/// Monomorphic phase enum for list-variable solutions (VRP, scheduling).
///
/// NO Box<dyn Phase> - all types are concrete. The compiler generates
/// optimized code paths for each variant at compile time.
pub enum ListPhaseImpl<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    /// Construction phase using ListChangeMoveSelector.
    Construction(ConstructionPhaseHolder<S, V, D>),
    /// Local search phase using ListMoveSelectorImpl.
    LocalSearch(LocalSearchPhaseHolder<S, V, D>),
}

/// Holder for construction phase with tracing.
pub struct ConstructionPhaseHolder<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    phase: ConstructionHeuristicPhase<
        S,
        MoveImpl<S, V>,
        crate::phase::construction::QueuedEntityPlacer<S, MoveImpl<S, V>>,
        ConstructionForagerImpl<S, MoveImpl<S, V>>,
    >,
    _phantom: PhantomData<D>,
}

impl<S, V, D> Debug for ConstructionPhaseHolder<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstructionPhaseHolder").finish()
    }
}

/// Holder for local search phase with tracing.
pub struct LocalSearchPhaseHolder<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    phase: LocalSearchPhase<
        S,
        MoveImpl<S, V>,
        MoveSelectorImpl<S, V>,
        AcceptorImpl<S>,
        ForagerImpl<S>,
    >,
    _phantom: PhantomData<D>,
}

impl<S, V, D> Debug for LocalSearchPhaseHolder<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSearchPhaseHolder").finish()
    }
}

impl<S, V, D> Debug for ListPhaseImpl<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Construction(p) => p.fmt(f),
            Self::LocalSearch(p) => p.fmt(f),
        }
    }
}

impl<S, V, D> Phase<S, D> for ListPhaseImpl<S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D>) {
        let start = Instant::now();
        let phase_name = self.phase_type_name();

        info!(event = "phase_start", phase = phase_name);

        match self {
            Self::Construction(holder) => holder.phase.solve(solver_scope),
            Self::LocalSearch(holder) => holder.phase.solve(solver_scope),
        }

        let duration = start.elapsed();
        let score = solver_scope
            .best_score()
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "none".to_string());

        info!(
            event = "phase_end",
            phase = phase_name,
            duration_ms = duration.as_millis() as u64,
            score = %score,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        match self {
            Self::Construction(_) => "Construction",
            Self::LocalSearch(_) => "LocalSearch",
        }
    }
}

// ============================================================================
// PhaseSequence - executes phases in order
// ============================================================================

/// A sequence of phases that execute in order.
///
/// NO Box<dyn Phase> - uses Vec of concrete enum type.
pub struct PhaseSequence<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    phases: Vec<ListPhaseImpl<S, V, D>>,
}

impl<S, V, D> Debug for PhaseSequence<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhaseSequence")
            .field("phase_count", &self.phases.len())
            .finish()
    }
}

impl<S, V, D> PhaseSequence<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    /// Creates an empty phase sequence.
    pub fn new() -> Self {
        Self { phases: Vec::new() }
    }

    /// Adds a phase to the sequence.
    pub fn add_phase(mut self, phase: ListPhaseImpl<S, V, D>) -> Self {
        self.phases.push(phase);
        self
    }

    /// Returns the number of phases.
    pub fn len(&self) -> usize {
        self.phases.len()
    }

    /// Returns true if the sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.phases.is_empty()
    }
}

impl<S, V, D> Default for PhaseSequence<S, V, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, V, D> Phase<S, D> for PhaseSequence<S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D>) {
        for phase in &mut self.phases {
            if solver_scope.is_terminate_early() {
                break;
            }
            phase.solve(solver_scope);
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseSequence"
    }
}

// ============================================================================
// PhaseFactory - creates phases from configuration
// ============================================================================

/// Factory for creating phases from configuration.
///
/// All created phases are concrete types - NO Box<dyn Phase>.
pub struct PhaseFactory<S, V>
where
    S: PlanningSolution,
{
    _phantom: PhantomData<(S, V)>,
}

impl<S, V> PhaseFactory<S, V>
where
    S: PlanningSolution + VariableOperations<Element = V>,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    S::Score: Score,
{
    /// Creates a construction phase from configuration.
    pub fn construction_phase<D: ScoreDirector<S>>(
        config: &ConstructionHeuristicConfig,
    ) -> ListPhaseImpl<S, V, D> {
        let forager_type = config
            .construction_heuristic_type
            .unwrap_or(ConstructionHeuristicType::FirstFit);

        let phase = ListConstructionPhaseBuilder::<S, V>::new(
            |s| s.element_count(),
            |s| s.assigned_elements(),
            |s| s.entity_count(),
            |s, entity_idx, elem| s.assign(entity_idx, elem),
            |idx| idx,
            S::variable_name(),
            S::descriptor_index(),
        )
        .with_forager(ConstructionForagerImpl::from_config(forager_type))
        .create_phase();

        ListPhaseImpl::Construction(ConstructionPhaseHolder {
            phase,
            _phantom: PhantomData,
        })
    }

    /// Creates a local search phase from configuration.
    pub fn local_search_phase<D: ScoreDirector<S>>(
        config: &LocalSearchConfig,
    ) -> ListPhaseImpl<S, V, D> {
        // Create move selector from config
        let move_selector = MoveSelectorImpl::<S, V>::from_config(config.move_selector.as_ref());

        // Create acceptor from config
        let acceptor = AcceptorImpl::<S>::from_config(config.acceptor.as_ref());

        // Create forager from config
        let forager = ForagerImpl::<S>::from_config(config.forager.as_ref());

        // Get step limit from termination config
        let step_limit = config
            .termination
            .as_ref()
            .and_then(|t| t.step_count_limit);

        let phase = LocalSearchPhase::new(move_selector, acceptor, forager, step_limit);

        ListPhaseImpl::LocalSearch(LocalSearchPhaseHolder {
            phase,
            _phantom: PhantomData,
        })
    }

    /// Creates a phase sequence from solver configuration.
    pub fn from_config<D: ScoreDirector<S>>(config: &SolverConfig) -> PhaseSequence<S, V, D> {
        let mut sequence = PhaseSequence::new();

        for phase_config in &config.phases {
            let phase = match phase_config {
                PhaseConfig::ConstructionHeuristic(cfg) => Self::construction_phase(cfg),
                PhaseConfig::LocalSearch(cfg) => Self::local_search_phase(cfg),
                PhaseConfig::ExhaustiveSearch(_) => {
                    panic!("ExhaustiveSearch phase not yet implemented - configure Construction or LocalSearch instead")
                }
                PhaseConfig::PartitionedSearch(_) => {
                    panic!("PartitionedSearch phase not yet implemented - configure Construction or LocalSearch instead")
                }
                PhaseConfig::Custom(_) => {
                    panic!("Custom phase not yet implemented - configure Construction or LocalSearch instead")
                }
            };
            sequence = sequence.add_phase(phase);
        }

        // If no phases configured, use defaults
        if sequence.is_empty() {
            sequence = sequence
                .add_phase(Self::construction_phase(&ConstructionHeuristicConfig::default()))
                .add_phase(Self::local_search_phase(&LocalSearchConfig::default()));
        }

        sequence
    }
}
