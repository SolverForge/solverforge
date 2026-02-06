//! Great Deluge acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::Acceptor;

/// Great Deluge acceptor - accepts moves above a rising water level.
///
/// The water level starts at the initial solution's score and rises over time.
/// A move is accepted if its resulting score is at or above the current water level.
/// This allows temporary score degradation while gradually tightening acceptance.
///
/// # Algorithm
///
/// 1. Water level starts at initial score
/// 2. Each step, water level rises by `rain_speed * |initial_score|`
/// 3. Accept if `move_score >= water_level`
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::GreatDelugeAcceptor;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_core::domain::PlanningSolution;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// // Rain speed of 0.001 means water level rises by 0.1% of |initial| per step
/// let acceptor = GreatDelugeAcceptor::<MySolution>::new(0.001);
/// ```
pub struct GreatDelugeAcceptor<S: PlanningSolution> {
    /// Rain speed - ratio of |initial_score| to add per step.
    rain_speed: f64,
    /// Current water level.
    water_level: Option<S::Score>,
    /// Absolute value of initial score, used to compute increment.
    initial_abs_score: Option<S::Score>,
}

impl<S: PlanningSolution> Debug for GreatDelugeAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GreatDelugeAcceptor")
            .field("rain_speed", &self.rain_speed)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for GreatDelugeAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            rain_speed: self.rain_speed,
            water_level: self.water_level,
            initial_abs_score: self.initial_abs_score,
        }
    }
}

impl<S: PlanningSolution> GreatDelugeAcceptor<S> {
    /// Creates a new Great Deluge acceptor.
    ///
    /// # Arguments
    /// * `rain_speed` - Ratio of |initial_score| to raise water level per step.
    ///   Typical values: 0.0001 to 0.01
    pub fn new(rain_speed: f64) -> Self {
        Self {
            rain_speed,
            water_level: None,
            initial_abs_score: None,
        }
    }
}

impl<S: PlanningSolution> Default for GreatDelugeAcceptor<S> {
    fn default() -> Self {
        Self::new(0.001)
    }
}

impl<S: PlanningSolution> Acceptor<S> for GreatDelugeAcceptor<S> {
    fn is_accepted(&mut self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Always accept improving moves
        if move_score > last_step_score {
            return true;
        }

        // Accept if at or above water level
        match &self.water_level {
            Some(water_level) => move_score >= water_level,
            None => true, // No water level yet, accept
        }
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.water_level = Some(*initial_score);
        self.initial_abs_score = Some(initial_score.abs());
    }

    fn step_ended(&mut self, _step_score: &S::Score) {
        // Raise water level by rain_speed * |initial_score|
        if let (Some(water), Some(abs_score)) = (&self.water_level, &self.initial_abs_score) {
            let increment = abs_score.multiply(self.rain_speed);
            self.water_level = Some(*water + increment);
        }
    }

    fn phase_ended(&mut self) {
        self.water_level = None;
        self.initial_abs_score = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::score::SimpleScore;

    #[derive(Clone)]
    struct TestSolution {
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[test]
    fn test_accepts_improving_moves() {
        let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.001);
        acceptor.phase_started(&SimpleScore::of(-100));

        // Improving move: -100 -> -50
        assert!(acceptor.is_accepted(&SimpleScore::of(-100), &SimpleScore::of(-50)));
    }

    #[test]
    fn test_accepts_above_water_level() {
        let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.001);
        acceptor.phase_started(&SimpleScore::of(-100));

        // Equal to water level
        assert!(acceptor.is_accepted(&SimpleScore::of(-100), &SimpleScore::of(-100)));

        // Above water level (less negative)
        assert!(acceptor.is_accepted(&SimpleScore::of(-100), &SimpleScore::of(-90)));
    }

    #[test]
    fn test_rejects_below_water_level() {
        let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.001);
        acceptor.phase_started(&SimpleScore::of(-100));

        // Below water level (more negative)
        assert!(!acceptor.is_accepted(&SimpleScore::of(-100), &SimpleScore::of(-110)));
    }

    #[test]
    fn test_water_rises_over_time() {
        let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.1);
        acceptor.phase_started(&SimpleScore::of(-100));

        // Initially water_level = -100
        // Score -100 is at water level (accepted)
        assert!(acceptor.is_accepted(&SimpleScore::of(-100), &SimpleScore::of(-100)));
        // Score -101 is below water level and not improving (-101 < -100)
        assert!(!acceptor.is_accepted(&SimpleScore::of(-100), &SimpleScore::of(-101)));

        // After one step, water rises by 0.1 * 100 = 10, so water_level = -90
        acceptor.step_ended(&SimpleScore::of(-100));
        // Score -90 is at water level
        assert!(acceptor.is_accepted(&SimpleScore::of(-90), &SimpleScore::of(-90)));
        // Score -91 is below water level and not improving (-91 < -90)
        assert!(!acceptor.is_accepted(&SimpleScore::of(-90), &SimpleScore::of(-91)));

        // After another step, water_level = -80
        acceptor.step_ended(&SimpleScore::of(-90));
        // Score -80 is at water level
        assert!(acceptor.is_accepted(&SimpleScore::of(-80), &SimpleScore::of(-80)));
        // Score -81 is below water level and not improving (-81 < -80)
        assert!(!acceptor.is_accepted(&SimpleScore::of(-80), &SimpleScore::of(-81)));
    }

    #[test]
    fn test_phase_reset() {
        let mut acceptor = GreatDelugeAcceptor::<TestSolution>::new(0.1);
        acceptor.phase_started(&SimpleScore::of(-100));
        acceptor.step_ended(&SimpleScore::of(-100));
        acceptor.phase_ended();

        // After phase ends, should reset
        acceptor.phase_started(&SimpleScore::of(-50));
        // Water level should be -50, not -90
        assert!(acceptor.is_accepted(&SimpleScore::of(-50), &SimpleScore::of(-50)));
        assert!(!acceptor.is_accepted(&SimpleScore::of(-50), &SimpleScore::of(-51)));
    }
}
