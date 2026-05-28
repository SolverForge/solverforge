//! Dynamic score support for binding models.

use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use solverforge_core::score::{ParseableScore, Score, ScoreLevel, ScoreParseError};

/// Declared host-language score family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DynamicScoreFamily {
    Soft,
    HardSoft,
    HardSoftDecimal,
    HardMediumSoft,
}

thread_local! {
    static ACTIVE_SCORE_FAMILY: Cell<DynamicScoreFamily> =
        const { Cell::new(DynamicScoreFamily::HardMediumSoft) };
}

/// Runs `callback` with a thread-local dynamic score family.
///
/// This keeps the internal static three-level score contract available to the
/// solver while allowing binding-driven solves to present zero and derived
/// scores with the host model's declared score family.
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

fn active_score_family() -> DynamicScoreFamily {
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
/// The static `Score` trait requires a static level count. The bridge therefore
/// stores one concrete three-level representation internally and carries the
/// declared host-language family for presentation and host-boundary conversion.
/// Binding crates must validate that a solve uses one declared family
/// consistently.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DynamicScore {
    pub hard: i64,
    pub medium: i64,
    pub soft: i64,
    pub family: DynamicScoreFamily,
}

impl DynamicScore {
    pub const ZERO: Self = Self {
        hard: 0,
        medium: 0,
        soft: 0,
        family: DynamicScoreFamily::HardMediumSoft,
    };

    pub const fn of(hard: i64, medium: i64, soft: i64) -> Self {
        Self {
            hard,
            medium,
            soft,
            family: DynamicScoreFamily::HardMediumSoft,
        }
    }

    pub const fn with_family(
        hard: i64,
        medium: i64,
        soft: i64,
        family: DynamicScoreFamily,
    ) -> Self {
        Self {
            hard,
            medium,
            soft,
            family,
        }
    }

    pub const fn soft(soft: i64) -> Self {
        Self::with_family(0, 0, soft, DynamicScoreFamily::Soft)
    }

    pub const fn hard_soft(hard: i64, soft: i64) -> Self {
        Self::with_family(hard, 0, soft, DynamicScoreFamily::HardSoft)
    }

    pub const fn hard_soft_decimal(hard_scaled: i64, soft_scaled: i64) -> Self {
        Self::with_family(
            hard_scaled,
            0,
            soft_scaled,
            DynamicScoreFamily::HardSoftDecimal,
        )
    }

    pub const fn hard_medium_soft(hard: i64, medium: i64, soft: i64) -> Self {
        Self::with_family(hard, medium, soft, DynamicScoreFamily::HardMediumSoft)
    }

    pub const fn zero_for_family(family: DynamicScoreFamily) -> Self {
        Self::with_family(0, 0, 0, family)
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

    fn is_zero(self) -> bool {
        self.hard == 0 && self.medium == 0 && self.soft == 0
    }

    fn combined_family(self, rhs: Self) -> DynamicScoreFamily {
        if self.family == rhs.family {
            return self.family;
        }
        if self.is_zero() {
            return rhs.family;
        }
        if rhs.is_zero() {
            return self.family;
        }
        DynamicScoreFamily::HardMediumSoft
    }
}

impl fmt::Debug for DynamicScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DynamicScore({}, {}, {}, {:?})",
            self.hard, self.medium, self.soft, self.family
        )
    }
}

impl fmt::Display for DynamicScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.family {
            DynamicScoreFamily::Soft => write!(f, "{}", self.soft),
            DynamicScoreFamily::HardSoft => write!(f, "{}hard/{}soft", self.hard, self.soft),
            DynamicScoreFamily::HardSoftDecimal => write!(
                f,
                "{}hard/{}soft",
                format_decimal_score_part(self.hard),
                format_decimal_score_part(self.soft)
            ),
            DynamicScoreFamily::HardMediumSoft => write!(
                f,
                "{}hard/{}medium/{}soft",
                self.hard, self.medium, self.soft
            ),
        }
    }
}

impl Ord for DynamicScore {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.hard, self.medium, self.soft).cmp(&(other.hard, other.medium, other.soft))
    }
}

impl PartialOrd for DynamicScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for DynamicScore {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::with_family(
            self.hard + rhs.hard,
            self.medium + rhs.medium,
            self.soft + rhs.soft,
            self.combined_family(rhs),
        )
    }
}

impl Sub for DynamicScore {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::with_family(
            self.hard - rhs.hard,
            self.medium - rhs.medium,
            self.soft - rhs.soft,
            self.combined_family(rhs),
        )
    }
}

impl Neg for DynamicScore {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::with_family(-self.hard, -self.medium, -self.soft, self.family)
    }
}

impl Score for DynamicScore {
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    fn zero() -> Self {
        Self::zero_for_family(active_score_family())
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
        match (active_score_family(), levels) {
            (DynamicScoreFamily::Soft, [soft]) => Self::soft(*soft),
            (DynamicScoreFamily::Soft, [_, _, soft]) => Self::soft(*soft),
            (DynamicScoreFamily::HardSoft, [hard, soft]) => Self::hard_soft(*hard, *soft),
            (DynamicScoreFamily::HardSoft, [hard, _, soft]) => Self::hard_soft(*hard, *soft),
            (DynamicScoreFamily::HardSoftDecimal, [hard, soft]) => {
                Self::hard_soft_decimal(*hard, *soft)
            }
            (DynamicScoreFamily::HardSoftDecimal, [hard, _, soft]) => {
                Self::hard_soft_decimal(*hard, *soft)
            }
            (DynamicScoreFamily::HardMediumSoft, [soft]) => Self::soft(*soft),
            (DynamicScoreFamily::HardMediumSoft, [hard, soft]) => Self::hard_soft(*hard, *soft),
            (DynamicScoreFamily::HardMediumSoft, [hard, medium, soft]) => {
                Self::hard_medium_soft(*hard, *medium, *soft)
            }
            _ => panic!("DynamicScore requires 1, 2, or 3 levels"),
        }
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        Self::with_family(
            (self.hard as f64 * multiplicand).round() as i64,
            (self.medium as f64 * multiplicand).round() as i64,
            (self.soft as f64 * multiplicand).round() as i64,
            self.family,
        )
    }

    fn divide(&self, divisor: f64) -> Self {
        self.multiply(1.0 / divisor)
    }

    fn abs(&self) -> Self {
        Self::with_family(
            self.hard.abs(),
            self.medium.abs(),
            self.soft.abs(),
            self.family,
        )
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
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let mut hard = 0;
        let mut medium = 0;
        let mut soft = 0;
        let mut has_hard = false;
        let mut has_medium = false;
        let mut has_soft = false;
        let mut hard_is_decimal = false;
        let mut soft_is_decimal = false;
        let mut is_decimal = false;
        for part in s.split('/') {
            if let Some(raw) = part.strip_suffix("hard") {
                has_hard = true;
                let (value, decimal) = parse_score_part(raw, "hard")?;
                hard = value;
                hard_is_decimal = decimal;
                is_decimal |= decimal;
            } else if let Some(raw) = part.strip_suffix("medium") {
                has_medium = true;
                let (value, decimal) = parse_score_part(raw, "medium")?;
                medium = value;
                is_decimal |= decimal;
            } else if let Some(raw) = part.strip_suffix("soft") {
                has_soft = true;
                let (value, decimal) = parse_score_part(raw, "soft")?;
                soft = value;
                soft_is_decimal = decimal;
                is_decimal |= decimal;
            } else if let Ok(value) = part.parse::<i64>() {
                soft = value;
            } else {
                return Err(ScoreParseError {
                    message: format!("invalid dynamic score `{s}`"),
                });
            }
        }
        if has_medium {
            Ok(Self::hard_medium_soft(hard, medium, soft))
        } else if has_hard || has_soft {
            if is_decimal {
                if has_hard && !hard_is_decimal {
                    hard *= DECIMAL_SCALE;
                }
                if has_soft && !soft_is_decimal {
                    soft *= DECIMAL_SCALE;
                }
                Ok(Self::hard_soft_decimal(hard, soft))
            } else {
                Ok(Self::hard_soft(hard, soft))
            }
        } else {
            Ok(Self::soft(soft))
        }
    }

    fn to_string_repr(&self) -> String {
        self.to_string()
    }
}

impl Default for DynamicScoreFamily {
    fn default() -> Self {
        Self::HardMediumSoft
    }
}

const DECIMAL_SCALE: i64 = 100_000;

fn format_decimal_score_part(scaled: i64) -> String {
    if scaled % DECIMAL_SCALE == 0 {
        return (scaled / DECIMAL_SCALE).to_string();
    }
    let value = scaled as f64 / DECIMAL_SCALE as f64;
    format!("{value:.6}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn parse_score_part(raw: &str, label: &str) -> Result<(i64, bool), ScoreParseError> {
    if raw.contains('.') {
        let value = raw.parse::<f64>().map_err(|_| ScoreParseError {
            message: format!("invalid {label} score `{raw}`"),
        })?;
        return Ok(((value * DECIMAL_SCALE as f64).round() as i64, true));
    }
    raw.parse::<i64>()
        .map(|value| (value, false))
        .map_err(|_| ScoreParseError {
            message: format!("invalid {label} score `{raw}`"),
        })
}
