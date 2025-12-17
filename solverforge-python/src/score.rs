//! Python bindings for score types.
//!
//! These types represent solution quality in constraint satisfaction problems.

use pyo3::prelude::*;
use pyo3::types::PyType;
use rust_decimal::Decimal;
use solverforge_core::{
    HardMediumSoftDecimalScore as RustHardMediumSoftDecimalScore,
    HardMediumSoftScore as RustHardMediumSoftScore,
    HardSoftDecimalScore as RustHardSoftDecimalScore, HardSoftScore as RustHardSoftScore,
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

/// A score with hard and soft decimal components.
///
/// Similar to HardSoftScore but uses decimal precision for fractional weights.
///
/// # Example
///
/// ```python
/// from solverforge import HardSoftDecimalScore
///
/// score = HardSoftDecimalScore.of(-2.5, 10.75)
/// assert score.hard_score == -2.5
/// assert score.soft_score == 10.75
/// assert not score.is_feasible()  # hard_score < 0
///
/// # Parse from string
/// score = HardSoftDecimalScore.parse("-5.5hard/10.25soft")
/// ```
#[pyclass(name = "HardSoftDecimalScore")]
#[derive(Clone, Debug)]
pub struct PyHardSoftDecimalScore {
    inner: RustHardSoftDecimalScore,
}

#[allow(non_snake_case)]
#[pymethods]
impl PyHardSoftDecimalScore {
    /// Create a new HardSoftDecimalScore.
    #[classmethod]
    fn of(_cls: &Bound<'_, PyType>, hard_score: f64, soft_score: f64) -> PyResult<Self> {
        let hard = Decimal::try_from(hard_score)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let soft = Decimal::try_from(soft_score)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            inner: RustHardSoftDecimalScore::of(hard, soft),
        })
    }

    /// Create a score with only a hard component.
    #[classmethod]
    fn of_hard(_cls: &Bound<'_, PyType>, hard_score: f64) -> PyResult<Self> {
        let hard = Decimal::try_from(hard_score)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            inner: RustHardSoftDecimalScore::of_hard(hard),
        })
    }

    /// Create a score with only a soft component.
    #[classmethod]
    fn of_soft(_cls: &Bound<'_, PyType>, soft_score: f64) -> PyResult<Self> {
        let soft = Decimal::try_from(soft_score)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            inner: RustHardSoftDecimalScore::of_soft(soft),
        })
    }

    /// Parse from string format like "-5.5hard/10.25soft".
    #[classmethod]
    fn parse(_cls: &Bound<'_, PyType>, text: &str) -> PyResult<Self> {
        RustHardSoftDecimalScore::parse(text)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// The zero score (0hard/0soft).
    #[classattr]
    fn ZERO() -> Self {
        Self {
            inner: RustHardSoftDecimalScore::zero(),
        }
    }

    /// One hard constraint penalty (1hard/0soft).
    #[classattr]
    fn ONE_HARD() -> Self {
        Self {
            inner: RustHardSoftDecimalScore::one_hard(),
        }
    }

    /// One soft constraint penalty (0hard/1soft).
    #[classattr]
    fn ONE_SOFT() -> Self {
        Self {
            inner: RustHardSoftDecimalScore::one_soft(),
        }
    }

    /// The hard score component.
    #[getter]
    fn hard_score(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.hard_score.to_f64().unwrap_or(0.0)
    }

    /// The soft score component.
    #[getter]
    fn soft_score(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.soft_score.to_f64().unwrap_or(0.0)
    }

    /// Whether this score is feasible (hard_score >= 0).
    #[getter]
    fn is_feasible(&self) -> bool {
        self.inner.hard_score >= Decimal::ZERO
    }

    fn __repr__(&self) -> String {
        format!(
            "HardSoftDecimalScore({}, {})",
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
            inner: RustHardSoftDecimalScore::of(
                self.inner.hard_score + other.inner.hard_score,
                self.inner.soft_score + other.inner.soft_score,
            ),
        }
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self {
            inner: RustHardSoftDecimalScore::of(
                self.inner.hard_score - other.inner.hard_score,
                self.inner.soft_score - other.inner.soft_score,
            ),
        }
    }

    fn __neg__(&self) -> Self {
        Self {
            inner: RustHardSoftDecimalScore::of(-self.inner.hard_score, -self.inner.soft_score),
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

impl PyHardSoftDecimalScore {
    pub fn from_rust(inner: RustHardSoftDecimalScore) -> Self {
        Self { inner }
    }

    pub fn to_rust(&self) -> RustHardSoftDecimalScore {
        self.inner
    }
}

/// A score with hard, medium, and soft decimal components.
///
/// Similar to HardMediumSoftScore but uses decimal precision for fractional weights.
///
/// # Example
///
/// ```python
/// from solverforge import HardMediumSoftDecimalScore
///
/// score = HardMediumSoftDecimalScore.of(-1.5, 5.0, 10.25)
/// assert not score.is_feasible()  # hard_score < 0
/// ```
#[pyclass(name = "HardMediumSoftDecimalScore")]
#[derive(Clone, Debug)]
pub struct PyHardMediumSoftDecimalScore {
    inner: RustHardMediumSoftDecimalScore,
}

#[allow(non_snake_case)]
#[pymethods]
impl PyHardMediumSoftDecimalScore {
    /// Create a new HardMediumSoftDecimalScore.
    #[classmethod]
    fn of(
        _cls: &Bound<'_, PyType>,
        hard_score: f64,
        medium_score: f64,
        soft_score: f64,
    ) -> PyResult<Self> {
        let hard = Decimal::try_from(hard_score)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let medium = Decimal::try_from(medium_score)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let soft = Decimal::try_from(soft_score)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            inner: RustHardMediumSoftDecimalScore::of(hard, medium, soft),
        })
    }

    /// Parse from string format like "-5.5hard/3.0medium/10.25soft".
    #[classmethod]
    fn parse(_cls: &Bound<'_, PyType>, text: &str) -> PyResult<Self> {
        RustHardMediumSoftDecimalScore::parse(text)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// The zero score.
    #[classattr]
    fn ZERO() -> Self {
        Self {
            inner: RustHardMediumSoftDecimalScore::zero(),
        }
    }

    /// One hard constraint penalty.
    #[classattr]
    fn ONE_HARD() -> Self {
        Self {
            inner: RustHardMediumSoftDecimalScore::one_hard(),
        }
    }

    /// One medium constraint penalty.
    #[classattr]
    fn ONE_MEDIUM() -> Self {
        Self {
            inner: RustHardMediumSoftDecimalScore::one_medium(),
        }
    }

    /// One soft constraint penalty.
    #[classattr]
    fn ONE_SOFT() -> Self {
        Self {
            inner: RustHardMediumSoftDecimalScore::one_soft(),
        }
    }

    /// The hard score component.
    #[getter]
    fn hard_score(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.hard_score.to_f64().unwrap_or(0.0)
    }

    /// The medium score component.
    #[getter]
    fn medium_score(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.medium_score.to_f64().unwrap_or(0.0)
    }

    /// The soft score component.
    #[getter]
    fn soft_score(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.inner.soft_score.to_f64().unwrap_or(0.0)
    }

    /// Whether this score is feasible (hard_score >= 0).
    #[getter]
    fn is_feasible(&self) -> bool {
        self.inner.hard_score >= Decimal::ZERO
    }

    fn __repr__(&self) -> String {
        format!(
            "HardMediumSoftDecimalScore({}, {}, {})",
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
            inner: RustHardMediumSoftDecimalScore::of(
                self.inner.hard_score + other.inner.hard_score,
                self.inner.medium_score + other.inner.medium_score,
                self.inner.soft_score + other.inner.soft_score,
            ),
        }
    }

    fn __sub__(&self, other: &Self) -> Self {
        Self {
            inner: RustHardMediumSoftDecimalScore::of(
                self.inner.hard_score - other.inner.hard_score,
                self.inner.medium_score - other.inner.medium_score,
                self.inner.soft_score - other.inner.soft_score,
            ),
        }
    }

    fn __neg__(&self) -> Self {
        Self {
            inner: RustHardMediumSoftDecimalScore::of(
                -self.inner.hard_score,
                -self.inner.medium_score,
                -self.inner.soft_score,
            ),
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

impl PyHardMediumSoftDecimalScore {
    pub fn from_rust(inner: RustHardMediumSoftDecimalScore) -> Self {
        Self { inner }
    }

    pub fn to_rust(&self) -> RustHardMediumSoftDecimalScore {
        self.inner
    }
}

/// Register score types with the Python module.
pub fn register_scores(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySimpleScore>()?;
    m.add_class::<PyHardSoftScore>()?;
    m.add_class::<PyHardMediumSoftScore>()?;
    m.add_class::<PyHardSoftDecimalScore>()?;
    m.add_class::<PyHardMediumSoftDecimalScore>()?;
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

    #[test]
    fn test_hard_soft_decimal_score() {
        let score = PyHardSoftDecimalScore {
            inner: RustHardSoftDecimalScore::of(
                Decimal::new(-25, 1),  // -2.5
                Decimal::new(1075, 2), // 10.75
            ),
        };
        assert!((score.hard_score() - (-2.5)).abs() < 0.001);
        assert!((score.soft_score() - 10.75).abs() < 0.001);
        assert!(!score.is_feasible());
    }

    #[test]
    fn test_hard_soft_decimal_score_feasible() {
        let feasible = PyHardSoftDecimalScore {
            inner: RustHardSoftDecimalScore::of(Decimal::ZERO, Decimal::new(-100, 0)),
        };
        assert!(feasible.is_feasible());
    }

    #[test]
    fn test_hard_soft_decimal_score_arithmetic() {
        let a = PyHardSoftDecimalScore {
            inner: RustHardSoftDecimalScore::of(Decimal::new(-20, 1), Decimal::new(100, 1)),
        };
        let b = PyHardSoftDecimalScore {
            inner: RustHardSoftDecimalScore::of(Decimal::new(-10, 1), Decimal::new(50, 1)),
        };

        let sum = a.__add__(&b);
        assert!((sum.hard_score() - (-3.0)).abs() < 0.001);
        assert!((sum.soft_score() - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_hard_medium_soft_decimal_score() {
        let score = PyHardMediumSoftDecimalScore {
            inner: RustHardMediumSoftDecimalScore::of(
                Decimal::new(-15, 1),  // -1.5
                Decimal::new(50, 1),   // 5.0
                Decimal::new(1025, 2), // 10.25
            ),
        };
        assert!((score.hard_score() - (-1.5)).abs() < 0.001);
        assert!((score.medium_score() - 5.0).abs() < 0.001);
        assert!((score.soft_score() - 10.25).abs() < 0.001);
        assert!(!score.is_feasible());
    }

    #[test]
    fn test_decimal_score_str() {
        let score = PyHardSoftDecimalScore {
            inner: RustHardSoftDecimalScore::of(Decimal::new(-55, 1), Decimal::new(1025, 2)),
        };
        assert_eq!(score.__str__(), "-5.5hard/10.25soft");
    }
}
