//! Local search phase factory.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::KOptMove;
use crate::heuristic::selector::entity::EntitySelector;
use crate::heuristic::selector::k_opt::{KOptConfig, ListPositionDistanceMeter, NearbyKOptMoveSelector};
use crate::heuristic::{Move, MoveSelector};
use crate::phase::localsearch::{
    AcceptedCountForager, Acceptor, HillClimbingAcceptor, LateAcceptanceAcceptor,
    LocalSearchForager, LocalSearchPhase, MoveTabuAcceptor, SimulatedAnnealingAcceptor,
    TabuSearchAcceptor, ValueTabuAcceptor,
};
use crate::phase::Phase;

use super::super::config::LocalSearchType;
use super::super::SolverPhaseFactory;

/// Factory for creating local search phases.
///
/// Local search phases improve an existing solution by exploring neighboring
/// solutions. The factory provides fresh phase instances for each solve,
/// ensuring that internal state (like tabu lists or temperature) is reset.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `M` - The move type (e.g., [`ChangeMove`](crate::heuristic::ChangeMove),
///   [`SwapMove`](crate::heuristic::SwapMove))
/// * `F` - The closure type that creates move selectors
///
/// # Available Acceptors
///
/// - [`hill_climbing`](Self::hill_climbing): Only accept improving moves
/// - [`tabu_search`](Self::tabu_search): Avoid recently visited states
/// - [`simulated_annealing`](Self::simulated_annealing): Probabilistic acceptance
/// - [`late_acceptance`](Self::late_acceptance): Compare against historical scores
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::{LocalSearchPhaseFactory, SolverPhaseFactory};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
/// impl PlanningSolution for S {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
/// fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
///
/// type M = ChangeMove<S, i32>;
///
/// // Hill climbing with step limit
/// let factory = LocalSearchPhaseFactory::<S, M, _>::hill_climbing(|| {
///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "value", vec![1, 2, 3]))
/// }).with_step_limit(1000);
///
/// // Tabu search
/// let tabu = LocalSearchPhaseFactory::<S, M, _>::tabu_search(7, || {
///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "value", vec![1, 2, 3]))
/// });
///
/// // Simulated annealing
/// let sa = LocalSearchPhaseFactory::<S, M, _>::simulated_annealing(1.0, 0.999, || {
///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "value", vec![1, 2, 3]))
/// });
/// ```
pub struct LocalSearchPhaseFactory<S, M, F>
where
    S: PlanningSolution,
    M: Move<S> + Clone + Send + Sync + 'static,
    F: Fn() -> Box<dyn MoveSelector<S, M>> + Send + Sync,
{
    search_type: LocalSearchType,
    step_limit: Option<u64>,
    move_selector_factory: F,
    _marker: PhantomData<(S, M)>,
}

impl<S, M, F> LocalSearchPhaseFactory<S, M, F>
where
    S: PlanningSolution,
    M: Move<S> + Clone + Send + Sync + 'static,
    F: Fn() -> Box<dyn MoveSelector<S, M>> + Send + Sync,
{
    /// Creates a new local search phase factory with the specified search type.
    ///
    /// # Arguments
    ///
    /// * `search_type` - The type of local search algorithm
    /// * `move_selector_factory` - A closure that creates move selectors
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::{LocalSearchType, LocalSearchPhaseFactory};
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// let factory = LocalSearchPhaseFactory::<S, M, _>::new(
    ///     LocalSearchType::TabuSearch { tabu_size: 10 },
    ///     || Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "v", vec![1, 2])),
    /// );
    /// ```
    pub fn new(search_type: LocalSearchType, move_selector_factory: F) -> Self {
        Self {
            search_type,
            step_limit: None,
            move_selector_factory,
            _marker: PhantomData,
        }
    }

    /// Sets the step limit for this phase.
    ///
    /// The phase will terminate after executing this many steps. This is useful
    /// for multi-phase configurations where you want to limit time spent in each phase.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchPhaseFactory;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// let factory = LocalSearchPhaseFactory::<S, M, _>::hill_climbing(|| {
    ///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "v", vec![1, 2]))
    /// }).with_step_limit(500);
    /// ```
    pub fn with_step_limit(mut self, limit: u64) -> Self {
        self.step_limit = Some(limit);
        self
    }

    /// Creates a factory with hill climbing acceptor.
    ///
    /// Hill climbing only accepts moves that improve the score. It is simple
    /// and fast, but can easily get stuck in local optima.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchPhaseFactory;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// let factory = LocalSearchPhaseFactory::<S, M, _>::hill_climbing(|| {
    ///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "value", vec![1, 2, 3]))
    /// });
    /// ```
    pub fn hill_climbing(move_selector_factory: F) -> Self {
        Self::new(LocalSearchType::HillClimbing, move_selector_factory)
    }

    /// Creates a factory with tabu search acceptor.
    ///
    /// Tabu search maintains a list of recently made moves and forbids
    /// reversing them. This helps escape local optima by forcing exploration.
    ///
    /// # Arguments
    ///
    /// * `tabu_size` - Number of recent moves to remember
    /// * `move_selector_factory` - Closure that creates move selectors
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchPhaseFactory;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// // Remember last 7 moves
    /// let factory = LocalSearchPhaseFactory::<S, M, _>::tabu_search(7, || {
    ///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "v", vec![1, 2, 3]))
    /// });
    /// ```
    pub fn tabu_search(tabu_size: usize, move_selector_factory: F) -> Self {
        Self::new(
            LocalSearchType::TabuSearch { tabu_size },
            move_selector_factory,
        )
    }

    /// Creates a factory with simulated annealing acceptor.
    ///
    /// Simulated annealing accepts worse moves with a probability that decreases
    /// over time. Initially, it explores widely; as the "temperature" cools,
    /// it becomes more selective.
    ///
    /// # Arguments
    ///
    /// * `starting_temp` - Initial temperature (higher = more exploration)
    /// * `decay_rate` - Rate at which temperature decreases (0.0 to 1.0)
    /// * `move_selector_factory` - Closure that creates move selectors
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchPhaseFactory;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// // Start at temperature 1.0, decay by 0.1% per step
    /// let factory = LocalSearchPhaseFactory::<S, M, _>::simulated_annealing(
    ///     1.0,
    ///     0.999,
    ///     || Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "v", vec![1, 2, 3])),
    /// );
    /// ```
    pub fn simulated_annealing(
        starting_temp: f64,
        decay_rate: f64,
        move_selector_factory: F,
    ) -> Self {
        Self::new(
            LocalSearchType::SimulatedAnnealing {
                starting_temp,
                decay_rate,
            },
            move_selector_factory,
        )
    }

    /// Creates a factory with late acceptance acceptor.
    ///
    /// Late acceptance compares the new score against the score from N steps ago.
    /// If the new score is better than or equal to that historical score, the move
    /// is accepted. This provides a balance between exploration and exploitation.
    ///
    /// # Arguments
    ///
    /// * `size` - Number of steps to look back
    /// * `move_selector_factory` - Closure that creates move selectors
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchPhaseFactory;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// // Compare against score from 400 steps ago
    /// let factory = LocalSearchPhaseFactory::<S, M, _>::late_acceptance(400, || {
    ///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "v", vec![1, 2, 3]))
    /// });
    /// ```
    pub fn late_acceptance(size: usize, move_selector_factory: F) -> Self {
        Self::new(
            LocalSearchType::LateAcceptance { size },
            move_selector_factory,
        )
    }

    /// Creates a factory with value tabu acceptor.
    ///
    /// Value tabu remembers recently assigned values and forbids reassigning them.
    /// This is different from entity tabu in that it tracks values, not entity-variable
    /// combinations.
    ///
    /// # Arguments
    ///
    /// * `value_tabu_size` - Number of recent values to forbid
    /// * `move_selector_factory` - Closure that creates move selectors
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchPhaseFactory;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// // Forbid last 5 assigned values
    /// let factory = LocalSearchPhaseFactory::<S, M, _>::value_tabu_search(5, || {
    ///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "v", vec![1, 2, 3]))
    /// });
    /// ```
    pub fn value_tabu_search(value_tabu_size: usize, move_selector_factory: F) -> Self {
        Self::new(
            LocalSearchType::ValueTabuSearch { value_tabu_size },
            move_selector_factory,
        )
    }

    /// Creates a factory with move tabu acceptor.
    ///
    /// Move tabu remembers recently made moves (by hash) and forbids making the same
    /// move again. Supports aspiration criterion: tabu moves can be accepted if they
    /// lead to a new best solution.
    ///
    /// # Arguments
    ///
    /// * `move_tabu_size` - Number of recent moves to forbid
    /// * `aspiration_enabled` - Whether to allow tabu moves that reach new best score
    /// * `move_selector_factory` - Closure that creates move selectors
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchPhaseFactory;
    /// use solverforge_solver::heuristic::r#move::ChangeMove;
    /// use solverforge_solver::heuristic::selector::ChangeMoveSelector;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)] struct S { values: Vec<Option<i32>>, score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// # fn get_v(s: &S, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
    /// # fn set_v(s: &mut S, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
    /// type M = ChangeMove<S, i32>;
    ///
    /// // Forbid last 10 moves, with aspiration enabled
    /// let factory = LocalSearchPhaseFactory::<S, M, _>::move_tabu_search(10, true, || {
    ///     Box::new(ChangeMoveSelector::<S, i32>::simple(get_v, set_v, 0, "v", vec![1, 2, 3]))
    /// });
    /// ```
    pub fn move_tabu_search(
        move_tabu_size: usize,
        aspiration_enabled: bool,
        move_selector_factory: F,
    ) -> Self {
        Self::new(
            LocalSearchType::MoveTabuSearch {
                move_tabu_size,
                aspiration_enabled,
            },
            move_selector_factory,
        )
    }

    fn create_acceptor(&self) -> Box<dyn Acceptor<S>> {
        match self.search_type {
            LocalSearchType::HillClimbing => Box::new(HillClimbingAcceptor::new()),
            LocalSearchType::TabuSearch { tabu_size } => {
                Box::new(TabuSearchAcceptor::new(tabu_size))
            }
            LocalSearchType::SimulatedAnnealing {
                starting_temp,
                decay_rate,
            } => Box::new(SimulatedAnnealingAcceptor::new(starting_temp, decay_rate)),
            LocalSearchType::LateAcceptance { size } => Box::new(LateAcceptanceAcceptor::new(size)),
            LocalSearchType::ValueTabuSearch { value_tabu_size } => {
                Box::new(ValueTabuAcceptor::new(value_tabu_size))
            }
            LocalSearchType::MoveTabuSearch {
                move_tabu_size,
                aspiration_enabled,
            } => {
                if aspiration_enabled {
                    Box::new(MoveTabuAcceptor::new(move_tabu_size))
                } else {
                    Box::new(MoveTabuAcceptor::without_aspiration(move_tabu_size))
                }
            }
        }
    }
}

impl<S, M, F> SolverPhaseFactory<S> for LocalSearchPhaseFactory<S, M, F>
where
    S: PlanningSolution + 'static,
    M: Move<S> + Clone + Send + Sync + 'static,
    F: Fn() -> Box<dyn MoveSelector<S, M>> + Send + Sync,
{
    fn create_phase(&self) -> Box<dyn Phase<S>> {
        let move_selector = (self.move_selector_factory)();
        let acceptor = self.create_acceptor();
        let forager: Box<dyn LocalSearchForager<S, M>> = Box::new(AcceptedCountForager::new(1));

        Box::new(LocalSearchPhase::new(
            move_selector,
            acceptor,
            forager,
            self.step_limit,
        ))
    }
}

/// Fluent builder for k-opt local search phases.
///
/// Configures nearby k-opt move selection with various acceptance strategies.
/// Uses distance-based pruning to reduce search space from O(n^k) to O(n × m^(k-1)).
///
/// # Type Parameters
///
/// * `S` - Planning solution type
/// * `V` - List element type (e.g., `usize` for visit indices)
/// * `D` - Distance meter for nearby selection
///
/// # Example
///
/// ```
/// use solverforge_solver::KOptPhaseBuilder;
/// use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
/// use solverforge_solver::heuristic::selector::k_opt::ListPositionDistanceMeter;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Route {
///     cities: Vec<usize>,
///     score: Option<SimpleScore>,
/// }
///
/// impl PlanningSolution for Route {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// #[derive(Debug, Clone, Copy)]
/// struct RouteMeter;
///
/// impl ListPositionDistanceMeter<Route> for RouteMeter {
///     fn distance(&self, route: &Route, _: usize, a: usize, b: usize) -> f64 {
///         // Simple position distance for demo
///         (a as f64 - b as f64).abs()
///     }
/// }
///
/// fn list_len(s: &Route, _: usize) -> usize { s.cities.len() }
/// fn sublist_remove(s: &mut Route, _: usize, start: usize, end: usize) -> Vec<usize> {
///     s.cities.drain(start..end).collect()
/// }
/// fn sublist_insert(s: &mut Route, _: usize, pos: usize, items: Vec<usize>) {
///     for (i, item) in items.into_iter().enumerate() {
///         s.cities.insert(pos + i, item);
///     }
/// }
///
/// let phase = KOptPhaseBuilder::<Route, usize, _>::new(
///     RouteMeter,
///     || Box::new(FromSolutionEntitySelector::new(0)),
///     list_len,
///     sublist_remove,
///     sublist_insert,
///     "cities",
///     0,
/// )
/// .with_k(3)
/// .with_nearby(10)
/// .with_late_acceptance(400);
/// ```
pub struct KOptPhaseBuilder<S, V, D>
where
    S: PlanningSolution,
    D: ListPositionDistanceMeter<S>,
{
    distance_meter: D,
    entity_selector_factory: Box<dyn Fn() -> Box<dyn EntitySelector<S>> + Send + Sync>,
    list_len: fn(&S, usize) -> usize,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    k: usize,
    max_nearby: usize,
    min_segment_len: usize,
    search_type: LocalSearchType,
    step_limit: Option<u64>,
    _phantom: PhantomData<(S, V)>,
}

impl<S, V, D> KOptPhaseBuilder<S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S> + Clone + 'static,
{
    /// Creates a new k-opt phase builder with required parameters.
    ///
    /// Defaults: k=3, max_nearby=10, min_segment_len=1, late acceptance with size 400.
    pub fn new<E>(
        distance_meter: D,
        entity_selector_factory: E,
        list_len: fn(&S, usize) -> usize,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self
    where
        E: Fn() -> Box<dyn EntitySelector<S>> + Send + Sync + 'static,
    {
        Self {
            distance_meter,
            entity_selector_factory: Box::new(entity_selector_factory),
            list_len,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            k: 3,
            max_nearby: 10,
            min_segment_len: 1,
            search_type: LocalSearchType::LateAcceptance { size: 400 },
            step_limit: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the k value (number of cuts). Must be 2-5.
    ///
    /// Higher k allows more complex restructuring but increases search space.
    /// - k=2: 2-opt (simple edge swap)
    /// - k=3: 3-opt (most common for VRP)
    /// - k=4, k=5: more complex moves, rarely needed
    pub fn with_k(mut self, k: usize) -> Self {
        assert!((2..=5).contains(&k), "k must be between 2 and 5");
        self.k = k;
        self
    }

    /// Sets the maximum nearby positions to consider for each cut.
    ///
    /// Lower values = faster but less thorough search.
    /// Higher values = slower but explores more combinations.
    /// Typical values: 5-20 for VRP.
    pub fn with_nearby(mut self, max_nearby: usize) -> Self {
        self.max_nearby = max_nearby;
        self
    }

    /// Sets the minimum segment length between cuts.
    ///
    /// Default is 1. Higher values prevent very short segments.
    pub fn with_min_segment_len(mut self, len: usize) -> Self {
        self.min_segment_len = len;
        self
    }

    /// Sets the step limit for this phase.
    pub fn with_step_limit(mut self, limit: u64) -> Self {
        self.step_limit = Some(limit);
        self
    }

    /// Uses hill climbing acceptance (only accept improving moves).
    pub fn with_hill_climbing(mut self) -> Self {
        self.search_type = LocalSearchType::HillClimbing;
        self
    }

    /// Uses tabu search acceptance.
    pub fn with_tabu_search(mut self, tabu_size: usize) -> Self {
        self.search_type = LocalSearchType::TabuSearch { tabu_size };
        self
    }

    /// Uses simulated annealing acceptance.
    pub fn with_simulated_annealing(mut self, starting_temp: f64, decay_rate: f64) -> Self {
        self.search_type = LocalSearchType::SimulatedAnnealing {
            starting_temp,
            decay_rate,
        };
        self
    }

    /// Uses late acceptance (compare against score from N steps ago).
    pub fn with_late_acceptance(mut self, size: usize) -> Self {
        self.search_type = LocalSearchType::LateAcceptance { size };
        self
    }

    fn create_acceptor(&self) -> Box<dyn Acceptor<S>> {
        match self.search_type {
            LocalSearchType::HillClimbing => Box::new(HillClimbingAcceptor::new()),
            LocalSearchType::TabuSearch { tabu_size } => {
                Box::new(TabuSearchAcceptor::new(tabu_size))
            }
            LocalSearchType::SimulatedAnnealing {
                starting_temp,
                decay_rate,
            } => Box::new(SimulatedAnnealingAcceptor::new(starting_temp, decay_rate)),
            LocalSearchType::LateAcceptance { size } => Box::new(LateAcceptanceAcceptor::new(size)),
            LocalSearchType::ValueTabuSearch { value_tabu_size } => {
                Box::new(ValueTabuAcceptor::new(value_tabu_size))
            }
            LocalSearchType::MoveTabuSearch {
                move_tabu_size,
                aspiration_enabled,
            } => {
                if aspiration_enabled {
                    Box::new(MoveTabuAcceptor::new(move_tabu_size))
                } else {
                    Box::new(MoveTabuAcceptor::without_aspiration(move_tabu_size))
                }
            }
        }
    }
}

impl<S, V, D> SolverPhaseFactory<S> for KOptPhaseBuilder<S, V, D>
where
    S: PlanningSolution + 'static,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S> + Clone + 'static,
{
    fn create_phase(&self) -> Box<dyn Phase<S>> {
        let config = KOptConfig::new(self.k).with_min_segment_len(self.min_segment_len);

        let move_selector: Box<dyn MoveSelector<S, KOptMove<S, V>>> =
            Box::new(NearbyKOptMoveSelector::new(
                (self.entity_selector_factory)(),
                self.distance_meter.clone(),
                self.max_nearby,
                config,
                self.list_len,
                self.sublist_remove,
                self.sublist_insert,
                self.variable_name,
                self.descriptor_index,
            ));

        let acceptor = self.create_acceptor();
        let forager: Box<dyn LocalSearchForager<S, KOptMove<S, V>>> =
            Box::new(AcceptedCountForager::new(1));

        Box::new(LocalSearchPhase::new(
            move_selector,
            acceptor,
            forager,
            self.step_limit,
        ))
    }
}
