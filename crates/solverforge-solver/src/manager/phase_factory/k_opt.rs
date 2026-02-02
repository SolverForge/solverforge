//! K-opt phase builder for tour optimization.
//!
//! Creates a local search phase that uses k-opt moves to improve solutions.
//! This is commonly used for vehicle routing and traveling salesman problems.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};

use crate::heuristic::selector::k_opt::{KOptConfig, KOptMoveSelector};
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

use super::super::PhaseFactory;

/// Builder for creating k-opt local search phases.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `V` - The list element value type (e.g., `usize` for visit indices)
/// * `DM` - The distance meter type (for nearby selection, stored for future use)
/// * `ESF` - Entity selector factory type (for nearby selection, stored for future use)
///
/// # Example
///
/// ```
/// use solverforge_solver::KOptPhaseBuilder;
/// use solverforge_solver::heuristic::selector::{DefaultDistanceMeter, FromSolutionEntitySelector};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Vehicle { visits: Vec<usize> }
///
/// #[derive(Clone, Debug)]
/// struct Plan {
///     vehicles: Vec<Vehicle>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for Plan {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Plan, idx: usize) -> usize {
///     s.vehicles.get(idx).map_or(0, |v| v.visits.len())
/// }
///
/// fn sublist_remove(s: &mut Plan, idx: usize, start: usize, end: usize) -> Vec<usize> {
///     s.vehicles.get_mut(idx)
///         .map(|v| v.visits.drain(start..end).collect())
///         .unwrap_or_default()
/// }
///
/// fn sublist_insert(s: &mut Plan, idx: usize, pos: usize, items: Vec<usize>) {
///     if let Some(v) = s.vehicles.get_mut(idx) {
///         for (i, item) in items.into_iter().enumerate() {
///             v.visits.insert(pos + i, item);
///         }
///     }
/// }
///
/// let builder = KOptPhaseBuilder::<Plan, usize, _, _>::new(
///     DefaultDistanceMeter,
///     || FromSolutionEntitySelector::new(0),
///     list_len,
///     sublist_remove,
///     sublist_insert,
///     "visits",
///     0,
/// );
/// ```
pub struct KOptPhaseBuilder<S, V, DM, ESF>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    list_len: fn(&S, usize) -> usize,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    k: usize,
    step_limit: Option<u64>,
    _marker: PhantomData<(S, V, DM, ESF)>,
}

impl<S, V, DM, ESF> KOptPhaseBuilder<S, V, DM, ESF>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    /// Creates a new k-opt phase builder.
    ///
    /// The `distance_meter` and `entity_selector_factory` parameters are accepted
    /// for API compatibility but not currently used (reserved for nearby selection).
    pub fn new(
        _distance_meter: DM,
        _entity_selector_factory: ESF,
        list_len: fn(&S, usize) -> usize,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            list_len,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            k: 3, // Default to 3-opt
            step_limit: Some(1000),
            _marker: PhantomData,
        }
    }

    /// Sets the k value for k-opt (2-5).
    pub fn with_k(mut self, k: usize) -> Self {
        assert!((2..=5).contains(&k), "k must be between 2 and 5");
        self.k = k;
        self
    }

    /// Sets the step limit.
    pub fn with_step_limit(mut self, limit: u64) -> Self {
        self.step_limit = Some(limit);
        self
    }

    /// Removes the step limit.
    pub fn without_step_limit(mut self) -> Self {
        self.step_limit = None;
        self
    }
}

impl<S, V, DM, ESF> Debug for KOptPhaseBuilder<S, V, DM, ESF>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KOptPhaseBuilder")
            .field("k", &self.k)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .field("step_limit", &self.step_limit)
            .finish()
    }
}

/// K-opt local search phase.
///
/// Iterates through k-opt moves and accepts improving ones using hill climbing.
pub struct KOptPhase<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    config: KOptConfig,
    list_len: fn(&S, usize) -> usize,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    step_limit: Option<u64>,
    _marker: PhantomData<(S, V)>,
}

impl<S, V> Debug for KOptPhase<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KOptPhase")
            .field("k", &self.config.k)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V, D> Phase<S, D> for KOptPhase<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        use crate::heuristic::r#move::Move;
        use crate::heuristic::selector::entity::FromSolutionEntitySelector;
        use crate::heuristic::selector::typed_move_selector::MoveSelector;

        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        // Calculate initial score
        let mut last_step_score = phase_scope.calculate_score();

        // Create move selector
        let entity_selector = FromSolutionEntitySelector::new(self.descriptor_index);
        let move_selector = KOptMoveSelector::<S, V, _>::new(
            entity_selector,
            self.config.clone(),
            self.list_len,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        );

        let step_limit = self.step_limit.unwrap_or(u64::MAX);
        let mut steps = 0u64;

        while steps < step_limit && !phase_scope.solver_scope().should_terminate() {
            let mut step_scope = StepScope::new(&mut phase_scope);

            // Collect moves first to avoid borrow conflicts
            let moves: Vec<_> = move_selector
                .iter_moves(step_scope.score_director())
                .collect();

            let mut best_move_idx = None;
            let mut best_score = None;

            // Evaluate all moves
            for (idx, mv) in moves.iter().enumerate() {
                if !mv.is_doable(step_scope.score_director()) {
                    continue;
                }

                // Use RecordingScoreDirector for automatic undo
                {
                    let mut recording =
                        RecordingScoreDirector::new(step_scope.score_director_mut());
                    mv.do_move(&mut recording);
                    let move_score = recording.calculate_score();

                    // Accept if improving over last step
                    if move_score > last_step_score
                        && best_score.as_ref().is_none_or(|b| move_score > *b)
                    {
                        best_score = Some(move_score);
                        best_move_idx = Some(idx);
                    }

                    // Undo the move
                    recording.undo_changes();
                }
            }

            // Apply best move if found
            if let (Some(idx), Some(score)) = (best_move_idx, best_score) {
                moves[idx].do_move(step_scope.score_director_mut());
                step_scope.set_step_score(score);
                last_step_score = score;
                step_scope.phase_scope_mut().update_best_solution();
            } else {
                // No improving moves found - phase is done
                break;
            }

            step_scope.complete();
            steps += 1;
        }

        // Ensure we have a best solution
        if phase_scope.solver_scope().best_solution().is_none() {
            phase_scope.update_best_solution();
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "KOpt"
    }
}

impl<S, V, DM, ESF, D> PhaseFactory<S, D> for KOptPhaseBuilder<S, V, DM, ESF>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    DM: Send + Sync,
    ESF: Send + Sync,
    D: ScoreDirector<S>,
{
    type Phase = KOptPhase<S, V>;

    fn create(&self) -> Self::Phase {
        KOptPhase {
            config: KOptConfig::new(self.k),
            list_len: self.list_len,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            step_limit: self.step_limit,
            _marker: PhantomData,
        }
    }
}
