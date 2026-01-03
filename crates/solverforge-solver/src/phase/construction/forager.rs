//! Foragers for construction heuristic move selection
//!
//! Foragers determine which move to select from the candidates
//! generated for each entity placement.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::heuristic::r#move::Move;

use super::Placement;

/// Trait for selecting a move during construction.
///
/// Foragers evaluate candidate moves and pick one based on their strategy.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait ConstructionForager<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Picks a move from the placement's candidates.
    ///
    /// Returns None if no suitable move is found.
    fn pick_move(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut dyn ScoreDirector<S>,
    ) -> Option<M>;
}

/// First Fit forager - picks the first feasible move.
///
/// This is the fastest forager but may not produce optimal results.
/// It simply takes the first move that can be executed.
#[derive(Clone, Default)]
pub struct FirstFitForager<S, M> {
    _phantom: PhantomData<(S, M)>,
}

impl<S, M> Debug for FirstFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstFitForager").finish()
    }
}

impl<S, M> FirstFitForager<S, M> {
    /// Creates a new First Fit forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for FirstFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut dyn ScoreDirector<S>,
    ) -> Option<M> {
        // Return the first doable move
        for m in &placement.moves {
            if m.is_doable(score_director) {
                return Some(m.clone());
            }
        }
        None
    }
}

/// Best Fit forager - evaluates all moves and picks the best.
///
/// This forager evaluates each candidate move by executing it,
/// calculating the score, and undoing it. The move with the best
/// score is selected.
#[derive(Clone, Default)]
pub struct BestFitForager<S, M> {
    _phantom: PhantomData<(S, M)>,
}

impl<S, M> Debug for BestFitForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestFitForager").finish()
    }
}

impl<S, M> BestFitForager<S, M> {
    /// Creates a new Best Fit forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for BestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut dyn ScoreDirector<S>,
    ) -> Option<M> {
        let mut best_move: Option<M> = None;
        let mut best_score: Option<S::Score> = None;

        for m in &placement.moves {
            if !m.is_doable(score_director) {
                continue;
            }

            // Use RecordingScoreDirector for automatic undo
            let score = {
                let mut recording = RecordingScoreDirector::new(score_director);

                // Execute move
                m.do_move(&mut recording);

                // Evaluate
                let score = recording.calculate_score();

                // Undo move
                recording.undo_changes();

                score
            };

            // Check if this is the best so far
            let is_better = match &best_score {
                None => true,
                Some(best) => score > *best,
            };

            if is_better {
                best_move = Some(m.clone());
                best_score = Some(score);
            }
        }

        best_move
    }
}

/// First Feasible forager - picks the first move that results in a feasible score.
///
/// This forager evaluates moves until it finds one that produces a feasible
/// (non-negative hard score) solution.
#[derive(Clone, Default)]
pub struct FirstFeasibleForager<S, M> {
    _phantom: PhantomData<(S, M)>,
}

impl<S, M> Debug for FirstFeasibleForager<S, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FirstFeasibleForager").finish()
    }
}

impl<S, M> FirstFeasibleForager<S, M> {
    /// Creates a new First Feasible forager.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for FirstFeasibleForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut dyn ScoreDirector<S>,
    ) -> Option<M> {
        let mut fallback_move: Option<M> = None;
        let mut fallback_score: Option<S::Score> = None;

        for m in &placement.moves {
            if !m.is_doable(score_director) {
                continue;
            }

            // Use RecordingScoreDirector for automatic undo
            let score = {
                let mut recording = RecordingScoreDirector::new(score_director);

                // Execute move
                m.do_move(&mut recording);

                // Evaluate
                let score = recording.calculate_score();

                // If feasible, return this move immediately
                if score.is_feasible() {
                    recording.undo_changes();
                    return Some(m.clone());
                }

                // Undo move
                recording.undo_changes();

                score
            };

            // Track best infeasible as fallback
            let is_better = match &fallback_score {
                None => true,
                Some(best) => score > *best,
            };

            if is_better {
                fallback_move = Some(m.clone());
                fallback_score = Some(score);
            }
        }

        // No feasible move found, return best infeasible
        fallback_move
    }
}

/// Weakest Fit forager - picks the move with the lowest strength value.
///
/// This forager evaluates each candidate move using a strength function
/// and selects the move with the minimum strength. This is useful for
/// assigning the "weakest" or least constraining values first.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::construction::{WeakestFitForager, ConstructionForager};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { priority: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Strength function: priority value (lower = weaker)
/// fn priority_strength(m: &ChangeMove<Solution, i32>) -> i64 {
///     m.to_value().map(|&v| v as i64).unwrap_or(0)
/// }
///
/// let forager = WeakestFitForager::<Solution, ChangeMove<Solution, i32>>::new(priority_strength);
/// ```
pub struct WeakestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Function to evaluate strength of a move.
    strength_fn: fn(&M) -> i64,
    _phantom: PhantomData<S>,
}

impl<S, M> Debug for WeakestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakestFitForager").finish()
    }
}

impl<S, M> WeakestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Creates a new Weakest Fit forager with the given strength function.
    ///
    /// The strength function evaluates how "strong" a move is. The forager
    /// picks the move with the minimum strength value.
    pub fn new(strength_fn: fn(&M) -> i64) -> Self {
        Self {
            strength_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for WeakestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut dyn ScoreDirector<S>,
    ) -> Option<M> {
        let mut best_move: Option<M> = None;
        let mut min_strength: Option<i64> = None;

        for m in &placement.moves {
            if !m.is_doable(score_director) {
                continue;
            }

            let strength = (self.strength_fn)(m);

            let is_weaker = match min_strength {
                None => true,
                Some(best) => strength < best,
            };

            if is_weaker {
                best_move = Some(m.clone());
                min_strength = Some(strength);
            }
        }

        best_move
    }
}

/// Strongest Fit forager - picks the move with the highest strength value.
///
/// This forager evaluates each candidate move using a strength function
/// and selects the move with the maximum strength. This is useful for
/// assigning the "strongest" or most constraining values first.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::construction::{StrongestFitForager, ConstructionForager};
/// use solverforge_solver::heuristic::r#move::ChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Task { priority: Option<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { tasks: Vec<Task>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Strength function: priority value (higher = stronger)
/// fn priority_strength(m: &ChangeMove<Solution, i32>) -> i64 {
///     m.to_value().map(|&v| v as i64).unwrap_or(0)
/// }
///
/// let forager = StrongestFitForager::<Solution, ChangeMove<Solution, i32>>::new(priority_strength);
/// ```
pub struct StrongestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Function to evaluate strength of a move.
    strength_fn: fn(&M) -> i64,
    _phantom: PhantomData<S>,
}

impl<S, M> Debug for StrongestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrongestFitForager").finish()
    }
}

impl<S, M> StrongestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Creates a new Strongest Fit forager with the given strength function.
    ///
    /// The strength function evaluates how "strong" a move is. The forager
    /// picks the move with the maximum strength value.
    pub fn new(strength_fn: fn(&M) -> i64) -> Self {
        Self {
            strength_fn,
            _phantom: PhantomData,
        }
    }
}

impl<S, M> ConstructionForager<S, M> for StrongestFitForager<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn pick_move(
        &self,
        placement: &Placement<S, M>,
        score_director: &mut dyn ScoreDirector<S>,
    ) -> Option<M> {
        let mut best_move: Option<M> = None;
        let mut max_strength: Option<i64> = None;

        for m in &placement.moves {
            if !m.is_doable(score_director) {
                continue;
            }

            let strength = (self.strength_fn)(m);

            let is_stronger = match max_strength {
                None => true,
                Some(best) => strength > best,
            };

            if is_stronger {
                best_move = Some(m.clone());
                max_strength = Some(strength);
            }
        }

        best_move
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::ChangeMove;
    use crate::heuristic::selector::EntityReference;
    use solverforge_scoring::SimpleScoreDirector;
    use solverforge_core::domain::{
        EntityDescriptor, SolutionDescriptor, TypedEntityExtractor,
    };
    use solverforge_core::score::SimpleScore;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Queen {
        row: Option<i64>,
    }

    #[derive(Clone, Debug)]
    struct NQueensSolution {
        queens: Vec<Queen>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for NQueensSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
        &s.queens
    }

    fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
        &mut s.queens
    }

    // Typed getter - zero erasure
    fn get_queen_row(s: &NQueensSolution, idx: usize) -> Option<i64> {
        s.queens.get(idx).and_then(|q| q.row)
    }

    // Typed setter - zero erasure
    fn set_queen_row(s: &mut NQueensSolution, idx: usize, v: Option<i64>) {
        if let Some(queen) = s.queens.get_mut(idx) {
            queen.row = v;
        }
    }

    fn create_test_director() -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
        let solution = NQueensSolution {
            queens: vec![Queen { row: None }],
            score: None,
        };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Queen",
            "queens",
            get_queens,
            get_queens_mut,
        ));let entity_desc = EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens")
            .with_extractor(extractor);

        let descriptor =
            SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
                .with_entity(entity_desc);

        // Score function: prefer higher row values
        SimpleScoreDirector::with_calculator(solution, descriptor, |sol| {
            let sum: i64 = sol.queens.iter().filter_map(|q| q.row).sum();
            SimpleScore::of(sum)
        })
    }

    type TestMove = ChangeMove<NQueensSolution, i64>;

    fn create_placement() -> Placement<NQueensSolution, TestMove> {
        let entity_ref = EntityReference::new(0, 0);
        let moves: Vec<TestMove> = vec![
            ChangeMove::new(0, Some(1i64), get_queen_row, set_queen_row, "row", 0),
            ChangeMove::new(0, Some(5i64), get_queen_row, set_queen_row, "row", 0),
            ChangeMove::new(0, Some(3i64), get_queen_row, set_queen_row, "row", 0),
        ];
        Placement::new(entity_ref, moves)
    }

    #[test]
    fn test_first_fit_forager() {
        let mut director = create_test_director();
        let placement = create_placement();

        let forager = FirstFitForager::<NQueensSolution, TestMove>::new();
        let selected = forager.pick_move(&placement, &mut director);

        // First Fit should pick the first move (value 1)
        assert!(selected.is_some());
    }

    #[test]
    fn test_best_fit_forager() {
        let mut director = create_test_director();
        let placement = create_placement();

        let forager = BestFitForager::<NQueensSolution, TestMove>::new();
        let selected = forager.pick_move(&placement, &mut director);

        // Best Fit should pick the move with highest score (value 5)
        assert!(selected.is_some());

        // Execute the selected move and check the score
        if let Some(m) = selected {
            m.do_move(&mut director);
            let score = director.calculate_score();
            assert_eq!(score, SimpleScore::of(5));
        }
    }

    #[test]
    fn test_empty_placement() {
        let mut director = create_test_director();
        let placement = Placement::new(EntityReference::new(0, 0), vec![]);

        let forager = FirstFitForager::<NQueensSolution, TestMove>::new();
        let selected = forager.pick_move(&placement, &mut director);

        assert!(selected.is_none());
    }

    fn value_strength(m: &TestMove) -> i64 {
        m.to_value().map(|&v| v).unwrap_or(0)
    }

    #[test]
    fn test_weakest_fit_forager() {
        let mut director = create_test_director();
        let placement = create_placement(); // values: 1, 5, 3

        let forager = WeakestFitForager::<NQueensSolution, TestMove>::new(value_strength);
        let selected = forager.pick_move(&placement, &mut director);

        // Weakest Fit should pick the move with lowest strength (value 1)
        assert!(selected.is_some());
        if let Some(m) = selected {
            m.do_move(&mut director);
            let score = director.calculate_score();
            assert_eq!(score, SimpleScore::of(1));
        }
    }

    #[test]
    fn test_strongest_fit_forager() {
        let mut director = create_test_director();
        let placement = create_placement(); // values: 1, 5, 3

        let forager = StrongestFitForager::<NQueensSolution, TestMove>::new(value_strength);
        let selected = forager.pick_move(&placement, &mut director);

        // Strongest Fit should pick the move with highest strength (value 5)
        assert!(selected.is_some());
        if let Some(m) = selected {
            m.do_move(&mut director);
            let score = director.calculate_score();
            assert_eq!(score, SimpleScore::of(5));
        }
    }
}
