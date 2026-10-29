//! Dynamic score support for binding models.

use std::cell::Cell;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use solverforge_core::score::{
    HardSoftDecimalScore, ParseableScore, Score, ScoreLevel, ScoreParseError,
};

/// Declared host-language score family.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DynamicScoreFamily {
    Soft,
    HardSoft,
    HardSoftDecimal,
    #[default]
    HardMediumSoft,
}

thread_local! {
    static ACTIVE_SCORE_FAMILY: Cell<DynamicScoreFamily> =
        const { Cell::new(DynamicScoreFamily::HardMediumSoft) };
}

/// Runs `callback` with a thread-local dynamic score family.
///
/// The family decides only how scores are presented and parsed: `Display`
/// renders the family's levels and `ParseableScore::parse` reads the family's
/// units. Score values and arithmetic do not depend on it, so scoring on any
/// thread, including parallel evaluation's worker threads, needs no scope.
/// Bindings enter the declared family around the solve so its output and
/// configured score limits use the host model's family.
pub fn scoped_dynamic_score_family<T>(
    family: DynamicScoreFamily,
    callback: impl FnOnce() -> T,
) -> T {
    ACTIVE_SCORE_FAMILY.with(|active| {
        let _guard = ActiveScoreFamilyGuard {
            previous: active.replace(family),
            active,
        };
        callback()
    })
}

/// The family the calling thread presents and parses scores with.
pub fn active_dynamic_score_family() -> DynamicScoreFamily {
    ACTIVE_SCORE_FAMILY.with(Cell::get)
}

struct ActiveScoreFamilyGuard<'a> {
    previous: DynamicScoreFamily,
    active: &'a Cell<DynamicScoreFamily>,
}

impl Drop for ActiveScoreFamilyGuard<'_> {
    fn drop(&mut self) {
        self.active.set(self.previous);
    }
}

/// Dynamic score used by host-language bindings.
///
/// The static `Score` trait requires a static level count, so every family
/// shares one three-level value: unused levels stay zero, and
/// `HardSoftDecimal` levels hold values scaled by 100 000. Equality, ordering,
/// and hashing compare the levels only. The declared family is a property of
/// the solve, not of the value; see [`scoped_dynamic_score_family`].
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DynamicScore {
    pub hard: i64,
    pub medium: i64,
    pub soft: i64,
}

impl DynamicScore {
    pub const ZERO: Self = Self::of(0, 0, 0);

    pub const fn of(hard: i64, medium: i64, soft: i64) -> Self {
        Self { hard, medium, soft }
    }

    pub const fn soft(soft: i64) -> Self {
        Self::of(0, 0, soft)
    }

    pub const fn hard_soft(hard: i64, soft: i64) -> Self {
        Self::of(hard, 0, soft)
    }

    pub fn family_levels(self, family: DynamicScoreFamily) -> Vec<i64> {
        match family {
            DynamicScoreFamily::Soft => vec![self.soft],
            DynamicScoreFamily::HardSoft | DynamicScoreFamily::HardSoftDecimal => {
                vec![self.hard, self.soft]
            }
            DynamicScoreFamily::HardMediumSoft => vec![self.hard, self.medium, self.soft],
        }
    }
}

impl fmt::Debug for DynamicScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DynamicScore({}, {}, {})",
            self.hard, self.medium, self.soft
        )
    }
}

impl fmt::Display for DynamicScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match active_dynamic_score_family() {
            DynamicScoreFamily::Soft => write!(f, "{}", self.soft),
            DynamicScoreFamily::HardSoft => write!(f, "{}hard/{}soft", self.hard, self.soft),
            DynamicScoreFamily::HardSoftDecimal => {
                write!(
                    f,
                    "{}",
                    HardSoftDecimalScore::of_scaled(self.hard, self.soft)
                )
            }
            DynamicScoreFamily::HardMediumSoft => write!(
                f,
                "{}hard/{}medium/{}soft",
                self.hard, self.medium, self.soft
            ),
        }
    }
}

impl Add for DynamicScore {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::of(
            self.hard + rhs.hard,
            self.medium + rhs.medium,
            self.soft + rhs.soft,
        )
    }
}

impl Sub for DynamicScore {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::of(
            self.hard - rhs.hard,
            self.medium - rhs.medium,
            self.soft - rhs.soft,
        )
    }
}

impl Neg for DynamicScore {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::of(-self.hard, -self.medium, -self.soft)
    }
}

impl Score for DynamicScore {
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    fn zero() -> Self {
        Self::ZERO
    }

    fn levels_count() -> usize {
        3
    }

    fn level_number(&self, index: usize) -> i64 {
        match index {
            0 => self.hard,
            1 => self.medium,
            2 => self.soft,
            _ => panic!("DynamicScore has 3 levels, got index {index}"),
        }
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        match levels {
            [soft] => Self::soft(*soft),
            [hard, soft] => Self::hard_soft(*hard, *soft),
            [hard, medium, soft] => Self::of(*hard, *medium, *soft),
            _ => panic!("DynamicScore requires 1, 2, or 3 levels"),
        }
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        Self::of(
            (self.hard as f64 * multiplicand).round() as i64,
            (self.medium as f64 * multiplicand).round() as i64,
            (self.soft as f64 * multiplicand).round() as i64,
        )
    }

    fn divide(&self, divisor: f64) -> Self {
        self.multiply(1.0 / divisor)
    }

    fn scale(&self, factor: i64) -> Self {
        let level = |value: i64| match value.checked_mul(factor) {
            Some(scaled) => scaled,
            None => panic!("score level {value} overflows i64 when scaled by {factor}"),
        };
        Self::of(level(self.hard), level(self.medium), level(self.soft))
    }

    fn abs(&self) -> Self {
        Self::of(self.hard.abs(), self.medium.abs(), self.soft.abs())
    }

    fn to_scalar(&self) -> f64 {
        self.hard as f64 * 1_000_000.0 + self.medium as f64 * 1_000.0 + self.soft as f64
    }

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 => ScoreLevel::Hard,
            1 => ScoreLevel::Medium,
            2 => ScoreLevel::Soft,
            _ => panic!("DynamicScore has 3 levels, got index {index}"),
        }
    }
}

impl ParseableScore for DynamicScore {
    /// Parses a score in the active family's units. A decimal literal
    /// (`1.5soft`) is always scaled; whole-number parts are scaled too when
    /// the active family is `HardSoftDecimal`, so parsing inverts `Display`.
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let decimal_family = active_dynamic_score_family() == DynamicScoreFamily::HardSoftDecimal;
        let mut hard = (0, false);
        let mut medium = None;
        let mut soft = (0, false);
        let mut has_decimal_literal = false;
        for part in s.split('/') {
            if let Some(raw) = part.strip_suffix("hard") {
                hard = parse_score_part(raw, "hard")?;
                has_decimal_literal |= hard.1;
            } else if let Some(raw) = part.strip_suffix("medium") {
                let (value, decimal) = parse_score_part(raw, "medium")?;
                medium = Some(value);
                has_decimal_literal |= decimal;
            } else if let Some(raw) = part.strip_suffix("soft") {
                soft = parse_score_part(raw, "soft")?;
                has_decimal_literal |= soft.1;
            } else if let Ok(value) = part.parse::<i64>() {
                soft = (value, false);
            } else {
                return Err(ScoreParseError {
                    message: format!("invalid dynamic score `{s}`"),
                });
            }
        }
        if let Some(medium) = medium {
            return Ok(Self::of(hard.0, medium, soft.0));
        }
        let scale = |(value, decimal): (i64, bool)| {
            if (decimal_family || has_decimal_literal) && !decimal {
                value
                    .checked_mul(DECIMAL_SCALE)
                    .ok_or_else(|| ScoreParseError {
                        message: format!("dynamic score `{s}` is out of range"),
                    })
            } else {
                Ok(value)
            }
        };
        Ok(Self::hard_soft(scale(hard)?, scale(soft)?))
    }

    fn to_string_repr(&self) -> String {
        self.to_string()
    }
}

const DECIMAL_SCALE: i64 = 100_000;

fn parse_score_part(raw: &str, label: &str) -> Result<(i64, bool), ScoreParseError> {
    if raw.contains('.') {
        return HardSoftDecimalScore::parse_level(raw)
            .map(|scaled| (scaled, true))
            .map_err(|error| ScoreParseError {
                message: format!("invalid {label} score `{raw}`: {}", error.message),
            });
    }
    raw.parse::<i64>()
        .map(|value| (value, false))
        .map_err(|_| ScoreParseError {
            message: format!("invalid {label} score `{raw}`"),
        })
}
