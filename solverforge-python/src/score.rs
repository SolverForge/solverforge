//! Python bindings for score types.
//!
//! These types represent solution quality in constraint satisfaction problems.

use pyo3::prelude::*;
use pyo3::types::PyType;
use solverforge_core::{
    HardMediumSoftScore as RustHardMediumSoftScore, HardSoftScore as RustHardSoftScore,
    SimpleScore as RustSimpleScore,
};

/// A simple score with a single numeric value.
///
/// # Example
///
/// ```python
/// from solverforge import SimpleScore
///
/// score = SimpleScore.of(-10)
/// assert score.score == -10
/// assert score.is_feasible()
/// ```
#[pyclass(name = "SimpleScore")]
#[derive(Clone, Debug)]
pub struct PySimpleScore {
    inner: RustSimpleScore,
}

#[pymethods]
impl PySimpleScore {
    /// Create a new SimpleScore with the given value.
    #[classmethod]
    fn of(_cls: &Bound<'_, PyType>, score: i64) -> Self {
        Self {
            inner: RustSimpleScore::of(score),
        }
    }

    /// The zero score.
    #[classattr]
    const ZERO: PySimpleScore = PySimpleScore {
        inner: RustSimpleScore::ZERO,
    };

    /// A score of 1.
    #[classattr]
    const ONE: PySimpleScore = PySimpleScore {
        inner: RustSimpleScore::ONE,
    };

    /// The score value.
    #[getter]
    fn score(&self) -> i64 {
        self.inner.score
    }

    /// Whether this score is feasible (>= 0).
    fn is_feasible(&self) -> bool {
        self.inner.score >= 0
    }

    fn __repr__(&self) -> String {
        format!("SimpleScore({})", self.inner.score)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __ne__(&self, other: &Self) -> bool {
        self.inner != other.inner
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.inner < other.inner
    }

    fn __le__(&self, other: &Self) -> bool {
        self.inner <= other.inner
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.inner > other.inner
    }

    fn __ge__(&self, other: &Self) -> bool {
        self.inner >= other.inner
    }

    fn __add__(&self, other: &Self) -> Self {
        Self {
            inner: RustSimpleScore::of(self.inner.score + other.inner.score),
        }
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self {
            inner: RustSimpleScore::of(self.inner.score - other.inner.score),
        }
    }

    fn __neg__(&self) -> Self {
        Self {
            inner: RustSimpleScore::of(-self.inner.score),
        }
    }

    fn __hash__(&self) -> u64 {
        self.inner.score as u64
    }
}

impl PySimpleScore {
    pub fn from_rust(inner: RustSimpleScore) -> Self {
        Self { inner }
    }

    pub fn to_rust(&self) -> RustSimpleScore {
        self.inner
    }
}

/// A score with hard and soft components.
///
/// Hard constraints must be satisfied for a solution to be feasible.
/// Soft constraints are optimized but violations don't make a solution infeasible.
///
/// # Example
///
/// ```python
/// from solverforge import HardSoftScore
///
/// score = HardSoftScore.of(-2, 10)
/// assert score.hard_score == -2
/// assert score.soft_score == 10
/// assert not score.is_feasible()  # hard_score < 0
///
/// # Use class constants for constraint weights
/// penalty = HardSoftScore.ONE_HARD  # 1hard/0soft
/// ```
#[pyclass(name = "HardSoftScore")]
#[derive(Clone, Debug)]
pub struct PyHardSoftScore {
    inner: RustHardSoftScore,
}

#[pymethods]
impl PyHardSoftScore {
    /// Create a new HardSoftScore.
    #[classmethod]
    fn of(_cls: &Bound<'_, PyType>, hard_score: i64, soft_score: i64) -> Self {
        Self {
            inner: RustHardSoftScore::of(hard_score, soft_score),
        }
    }

    /// Create a score with only a hard component.
    #[classmethod]
    fn of_hard(_cls: &Bound<'_, PyType>, hard_score: i64) -> Self {
        Self {
            inner: RustHardSoftScore::of_hard(hard_score),
        }
    }

    /// Create a score with only a soft component.
    #[classmethod]
    fn of_soft(_cls: &Bound<'_, PyType>, soft_score: i64) -> Self {
        Self {
            inner: RustHardSoftScore::of_soft(soft_score),
        }
    }

    /// The zero score (0hard/0soft).
    #[classattr]
    const ZERO: PyHardSoftScore = PyHardSoftScore {
        inner: RustHardSoftScore::ZERO,
    };

    /// One hard constraint penalty (1hard/0soft).
    #[classattr]
    const ONE_HARD: PyHardSoftScore = PyHardSoftScore {
        inner: RustHardSoftScore::ONE_HARD,
    };

    /// One soft constraint penalty (0hard/1soft).
    #[classattr]
    const ONE_SOFT: PyHardSoftScore = PyHardSoftScore {
        inner: RustHardSoftScore::ONE_SOFT,
    };

    /// The hard score component.
    #[getter]
    fn hard_score(&self) -> i64 {
        self.inner.hard_score
    }

    /// The soft score component.
    #[getter]
    fn soft_score(&self) -> i64 {
        self.inner.soft_score
    }

    /// Whether this score is feasible (hard_score >= 0).
    fn is_feasible(&self) -> bool {
        self.inner.hard_score >= 0
    }

    fn __repr__(&self) -> String {
        format!(
            "HardSoftScore({}, {})",
            self.inner.hard_score, self.inner.soft_score
        )
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __ne__(&self, other: &Self) -> bool {
        self.inner != other.inner
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.inner < other.inner
    }

    fn __le__(&self, other: &Self) -> bool {
        self.inner <= other.inner
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.inner > other.inner
    }

    fn __ge__(&self, other: &Self) -> bool {
        self.inner >= other.inner
    }

    fn __add__(&self, other: &Self) -> Self {
        Self {
            inner: RustHardSoftScore::of(
                self.inner.hard_score + other.inner.hard_score,
                self.inner.soft_score + other.inner.soft_score,
            ),
        }
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self {
            inner: RustHardSoftScore::of(
                self.inner.hard_score - other.inner.hard_score,
                self.inner.soft_score - other.inner.soft_score,
            ),
        }
    }

    fn __neg__(&self) -> Self {
        Self {
            inner: RustHardSoftScore::of(-self.inner.hard_score, -self.inner.soft_score),
        }
    }

    fn __hash__(&self) -> u64 {
        let h = self.inner.hard_score as u64;
        let s = self.inner.soft_score as u64;
        h.wrapping_mul(31).wrapping_add(s)
    }
}

impl PyHardSoftScore {
    pub fn from_rust(inner: RustHardSoftScore) -> Self {
        Self { inner }
    }

    pub fn to_rust(&self) -> RustHardSoftScore {
        self.inner
    }
}

/// A score with hard, medium, and soft components.
///
/// Hard constraints must be satisfied for feasibility.
/// Medium constraints are prioritized over soft constraints.
/// Soft constraints are lowest priority optimizations.
///
/// # Example
///
/// ```python
/// from solverforge import HardMediumSoftScore
///
/// score = HardMediumSoftScore.of(-1, 5, 10)
/// assert not score.is_feasible()  # hard_score < 0
/// ```
#[pyclass(name = "HardMediumSoftScore")]
#[derive(Clone, Debug)]
pub struct PyHardMediumSoftScore {
    inner: RustHardMediumSoftScore,
}

#[pymethods]
impl PyHardMediumSoftScore {
    /// Create a new HardMediumSoftScore.
    #[classmethod]
    fn of(_cls: &Bound<'_, PyType>, hard_score: i64, medium_score: i64, soft_score: i64) -> Self {
        Self {
            inner: RustHardMediumSoftScore::of(hard_score, medium_score, soft_score),
        }
    }

    /// Create a score with only a hard component.
    #[classmethod]
    fn of_hard(_cls: &Bound<'_, PyType>, hard_score: i64) -> Self {
        Self {
            inner: RustHardMediumSoftScore::of_hard(hard_score),
        }
    }

    /// Create a score with only a medium component.
    #[classmethod]
    fn of_medium(_cls: &Bound<'_, PyType>, medium_score: i64) -> Self {
        Self {
            inner: RustHardMediumSoftScore::of_medium(medium_score),
        }
    }

    /// Create a score with only a soft component.
    #[classmethod]
    fn of_soft(_cls: &Bound<'_, PyType>, soft_score: i64) -> Self {
        Self {
            inner: RustHardMediumSoftScore::of_soft(soft_score),
        }
    }

    /// The zero score (0hard/0medium/0soft).
    #[classattr]
    const ZERO: PyHardMediumSoftScore = PyHardMediumSoftScore {
        inner: RustHardMediumSoftScore::ZERO,
    };

    /// One hard constraint penalty (1hard/0medium/0soft).
    #[classattr]
    const ONE_HARD: PyHardMediumSoftScore = PyHardMediumSoftScore {
        inner: RustHardMediumSoftScore::ONE_HARD,
    };

    /// One medium constraint penalty (0hard/1medium/0soft).
    #[classattr]
    const ONE_MEDIUM: PyHardMediumSoftScore = PyHardMediumSoftScore {
        inner: RustHardMediumSoftScore::ONE_MEDIUM,
    };

    /// One soft constraint penalty (0hard/0medium/1soft).
    #[classattr]
    const ONE_SOFT: PyHardMediumSoftScore = PyHardMediumSoftScore {
        inner: RustHardMediumSoftScore::ONE_SOFT,
    };

    /// The hard score component.
    #[getter]
    fn hard_score(&self) -> i64 {
        self.inner.hard_score
    }

    /// The medium score component.
    #[getter]
    fn medium_score(&self) -> i64 {
        self.inner.medium_score
    }

    /// The soft score component.
    #[getter]
    fn soft_score(&self) -> i64 {
        self.inner.soft_score
    }

    /// Whether this score is feasible (hard_score >= 0).
    fn is_feasible(&self) -> bool {
        self.inner.hard_score >= 0
    }

    fn __repr__(&self) -> String {
        format!(
            "HardMediumSoftScore({}, {}, {})",
            self.inner.hard_score, self.inner.medium_score, self.inner.soft_score
        )
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __ne__(&self, other: &Self) -> bool {
        self.inner != other.inner
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.inner < other.inner
    }

    fn __le__(&self, other: &Self) -> bool {
        self.inner <= other.inner
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.inner > other.inner
    }

    fn __ge__(&self, other: &Self) -> bool {
        self.inner >= other.inner
    }

    fn __add__(&self, other: &Self) -> Self {
        Self {
            inner: RustHardMediumSoftScore::of(
                self.inner.hard_score + other.inner.hard_score,
                self.inner.medium_score + other.inner.medium_score,
                self.inner.soft_score + other.inner.soft_score,
            ),
        }
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self {
            inner: RustHardMediumSoftScore::of(
                self.inner.hard_score - other.inner.hard_score,
                self.inner.medium_score - other.inner.medium_score,
                self.inner.soft_score - other.inner.soft_score,
            ),
        }
    }

    fn __neg__(&self) -> Self {
        Self {
            inner: RustHardMediumSoftScore::of(
                -self.inner.hard_score,
                -self.inner.medium_score,
                -self.inner.soft_score,
            ),
        }
    }

    fn __hash__(&self) -> u64 {
        let h = self.inner.hard_score as u64;
        let m = self.inner.medium_score as u64;
        let s = self.inner.soft_score as u64;
        h.wrapping_mul(31)
            .wrapping_add(m)
            .wrapping_mul(31)
            .wrapping_add(s)
    }
}

impl PyHardMediumSoftScore {
    pub fn from_rust(inner: RustHardMediumSoftScore) -> Self {
        Self { inner }
    }

    pub fn to_rust(&self) -> RustHardMediumSoftScore {
        self.inner
    }
}

/// Register score types with the Python module.
pub fn register_scores(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySimpleScore>()?;
    m.add_class::<PyHardSoftScore>()?;
    m.add_class::<PyHardMediumSoftScore>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_score() {
        let score = PySimpleScore {
            inner: RustSimpleScore::of(-10),
        };
        assert_eq!(score.score(), -10);
        assert!(!score.is_feasible());
    }

    #[test]
    fn test_hard_soft_score_of() {
        let score = PyHardSoftScore {
            inner: RustHardSoftScore::of(-2, 10),
        };
        assert_eq!(score.hard_score(), -2);
        assert_eq!(score.soft_score(), 10);
        assert!(!score.is_feasible());
    }

    #[test]
    fn test_hard_soft_score_feasible() {
        let feasible = PyHardSoftScore {
            inner: RustHardSoftScore::of(0, -100),
        };
        assert!(feasible.is_feasible());
    }

    #[test]
    fn test_hard_soft_score_comparison() {
        let a = PyHardSoftScore {
            inner: RustHardSoftScore::of(0, 10),
        };
        let b = PyHardSoftScore {
            inner: RustHardSoftScore::of(0, 5),
        };
        assert!(a.__gt__(&b));
        assert!(b.__lt__(&a));
    }

    #[test]
    fn test_hard_soft_score_arithmetic() {
        let a = PyHardSoftScore {
            inner: RustHardSoftScore::of(-2, 10),
        };
        let b = PyHardSoftScore {
            inner: RustHardSoftScore::of(-1, 5),
        };

        let sum = a.__add__(&b);
        assert_eq!(sum.hard_score(), -3);
        assert_eq!(sum.soft_score(), 15);

        let diff = a.__sub__(&b);
        assert_eq!(diff.hard_score(), -1);
        assert_eq!(diff.soft_score(), 5);

        let neg = a.__neg__();
        assert_eq!(neg.hard_score(), 2);
        assert_eq!(neg.soft_score(), -10);
    }

    #[test]
    fn test_hard_medium_soft_score() {
        let score = PyHardMediumSoftScore {
            inner: RustHardMediumSoftScore::of(-1, 5, 10),
        };
        assert_eq!(score.hard_score(), -1);
        assert_eq!(score.medium_score(), 5);
        assert_eq!(score.soft_score(), 10);
        assert!(!score.is_feasible());
    }

    #[test]
    fn test_hard_medium_soft_score_comparison() {
        let a = PyHardMediumSoftScore {
            inner: RustHardMediumSoftScore::of(0, 1, 0),
        };
        let b = PyHardMediumSoftScore {
            inner: RustHardMediumSoftScore::of(0, 0, 100),
        };
        // Medium takes precedence over soft
        assert!(a.__gt__(&b));
    }

    #[test]
    fn test_repr_and_str() {
        let score = PyHardSoftScore {
            inner: RustHardSoftScore::of(-5, 10),
        };
        assert_eq!(score.__repr__(), "HardSoftScore(-5, 10)");
        assert_eq!(score.__str__(), "-5hard/10soft");
    }
}
