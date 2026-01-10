//! Configuration types for solver phases.
//!
//! This module contains enums for configuring different types of solver phases:
//!
//! - [`LocalSearchType`]: Algorithm selection for local search phases
//! - [`ConstructionType`]: Algorithm selection for construction heuristic phases
//! - [`PhaseConfig`]: Complete phase configuration

/// Type of local search algorithm to use.
///
/// Different local search algorithms have different characteristics:
///
/// - [`HillClimbing`](Self::HillClimbing): Simple, fast, but can get stuck in local optima
/// - [`TabuSearch`](Self::TabuSearch): Avoids revisiting recent states
/// - [`SimulatedAnnealing`](Self::SimulatedAnnealing): Probabilistic acceptance of worse moves
/// - [`LateAcceptance`](Self::LateAcceptance): Compares against historical scores
///
/// # Examples
///
/// ```
/// use solverforge_solver::manager::LocalSearchType;
///
/// // Hill climbing - simplest approach
/// let hill = LocalSearchType::HillClimbing;
/// assert_eq!(hill, LocalSearchType::default());
///
/// // Tabu search with memory of 10 recent moves
/// let tabu = LocalSearchType::TabuSearch { tabu_size: 10 };
///
/// // Simulated annealing with temperature decay
/// let sa = LocalSearchType::SimulatedAnnealing {
///     starting_temp: 1.0,
///     decay_rate: 0.995,
/// };
///
/// // Late acceptance comparing to 100 steps ago
/// let late = LocalSearchType::LateAcceptance { size: 100 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocalSearchType {
    /// Hill climbing: only accept improving moves.
    ///
    /// This is the simplest local search strategy. It only accepts moves
    /// that improve the score. Fast but can easily get stuck in local optima.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchType;
    ///
    /// let acceptor = LocalSearchType::HillClimbing;
    /// ```
    HillClimbing,

    /// Tabu search: avoid recently visited states.
    ///
    /// Maintains a list of recently made moves and forbids reversing them.
    /// This helps escape local optima by forcing exploration.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchType;
    ///
    /// // Keep last 7 moves in tabu list
    /// let tabu = LocalSearchType::TabuSearch { tabu_size: 7 };
    /// ```
    TabuSearch {
        /// Size of the tabu list.
        tabu_size: usize,
    },

    /// Simulated annealing: accept worse moves with decreasing probability.
    ///
    /// Initially accepts worse moves with high probability, but as the
    /// "temperature" decreases, it becomes more selective. Good for
    /// escaping local optima early in the search.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchType;
    ///
    /// // Start with temperature 1.0, decay by 0.1% per step
    /// let sa = LocalSearchType::SimulatedAnnealing {
    ///     starting_temp: 1.0,
    ///     decay_rate: 0.999,
    /// };
    /// ```
    SimulatedAnnealing {
        /// Initial temperature.
        starting_temp: f64,
        /// Temperature decay rate per step.
        decay_rate: f64,
    },

    /// Late acceptance: compare against score from N steps ago.
    ///
    /// Accepts a move if the new score is better than the score from
    /// N steps ago. Provides a balance between exploration and exploitation.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchType;
    ///
    /// // Compare against score from 400 steps ago
    /// let late = LocalSearchType::LateAcceptance { size: 400 };
    /// ```
    LateAcceptance {
        /// Number of steps to look back.
        size: usize,
    },

    /// Value tabu search: forbid recently assigned values.
    ///
    /// Remembers recently assigned values and forbids reassigning them.
    /// Different from entity tabu in that it tracks the values themselves,
    /// not the entity-variable combinations.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchType;
    ///
    /// // Forbid last 5 assigned values
    /// let value_tabu = LocalSearchType::ValueTabuSearch { value_tabu_size: 5 };
    /// ```
    ValueTabuSearch {
        /// Number of recent values to forbid.
        value_tabu_size: usize,
    },

    /// Move tabu search: forbid recently made moves.
    ///
    /// Remembers recently made moves (by hash) and forbids making the same
    /// move again. Supports aspiration criterion: tabu moves can be accepted
    /// if they lead to a new best solution.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchType;
    ///
    /// // Forbid last 10 moves, with aspiration enabled
    /// let move_tabu = LocalSearchType::MoveTabuSearch {
    ///     move_tabu_size: 10,
    ///     aspiration_enabled: true,
    /// };
    /// ```
    MoveTabuSearch {
        /// Number of recent moves to forbid.
        move_tabu_size: usize,
        /// Whether to allow tabu moves that reach new best score.
        aspiration_enabled: bool,
    },
}

impl Default for LocalSearchType {
    /// Returns [`HillClimbing`](Self::HillClimbing) as the default.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::LocalSearchType;
    ///
    /// let default = LocalSearchType::default();
    /// assert_eq!(default, LocalSearchType::HillClimbing);
    /// ```
    fn default() -> Self {
        LocalSearchType::HillClimbing
    }
}

/// Type of construction heuristic to use.
///
/// Construction heuristics build an initial solution by assigning values
/// to uninitialized planning variables. The type determines how values
/// are selected:
///
/// - [`FirstFit`](Self::FirstFit): Fast, takes first valid value
/// - [`BestFit`](Self::BestFit): Slower, evaluates all options to find best
///
/// # Examples
///
/// ```
/// use solverforge_solver::manager::ConstructionType;
///
/// // First fit - faster but may produce lower quality initial solution
/// let fast = ConstructionType::FirstFit;
/// assert_eq!(fast, ConstructionType::default());
///
/// // Best fit - slower but produces better initial solution
/// let best = ConstructionType::BestFit;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConstructionType {
    /// First fit: accept first valid assignment.
    ///
    /// Assigns the first value that produces a feasible solution.
    /// Fast but may not produce an optimal initial solution.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::ConstructionType;
    ///
    /// let construction = ConstructionType::FirstFit;
    /// ```
    #[default]
    FirstFit,

    /// Best fit: evaluate all options, pick best score.
    ///
    /// Evaluates all possible values and selects the one that produces
    /// the best score. Slower but produces a higher quality initial solution.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::ConstructionType;
    ///
    /// let construction = ConstructionType::BestFit;
    /// ```
    BestFit,
}

/// Configuration for a phase.
///
/// This enum represents the configuration for different types of solver phases.
/// Use this with the builder to configure your solving strategy.
///
/// # Examples
///
/// ```
/// use solverforge_solver::manager::{PhaseConfig, ConstructionType, LocalSearchType};
///
/// // Construction phase configuration
/// let construction = PhaseConfig::ConstructionHeuristic {
///     construction_type: ConstructionType::BestFit,
/// };
///
/// // Local search phase with step limit
/// let local_search = PhaseConfig::LocalSearch {
///     search_type: LocalSearchType::TabuSearch { tabu_size: 7 },
///     step_limit: Some(1000),
/// };
/// ```
#[derive(Debug, Clone)]
pub enum PhaseConfig {
    /// Construction heuristic phase.
    ///
    /// Builds an initial solution by assigning values to uninitialized
    /// planning variables.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::{PhaseConfig, ConstructionType};
    ///
    /// let config = PhaseConfig::ConstructionHeuristic {
    ///     construction_type: ConstructionType::FirstFit,
    /// };
    /// ```
    ConstructionHeuristic {
        /// Type of construction.
        construction_type: ConstructionType,
    },

    /// Local search phase.
    ///
    /// Improves an existing solution by exploring neighboring solutions.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::{PhaseConfig, LocalSearchType};
    ///
    /// let config = PhaseConfig::LocalSearch {
    ///     search_type: LocalSearchType::HillClimbing,
    ///     step_limit: Some(500),
    /// };
    /// ```
    LocalSearch {
        /// Type of local search.
        search_type: LocalSearchType,
        /// Optional step limit for this phase.
        step_limit: Option<u64>,
    },
}
