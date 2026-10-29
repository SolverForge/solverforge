/* HardSoftDecimalScore - Two-level score with i64 precision and x100000 scaling

This score type represents a decimal score without heap allocation.
Internal values are scaled by 100000 to provide 5 decimal places of precision.
*/

use std::cmp::Ordering;
use std::fmt;

use super::traits::{ParseableScore, Score, ScoreParseError};
use super::ScoreLevel;

// Scale factor for 5 decimal places of precision.
const SCALE: i64 = 100_000;

/* A score with separate hard and soft constraint levels, using i64 with x100000 scaling.

This provides 5 decimal places of precision while maintaining zero heap allocation
and full type safety.

Internal values are stored pre-scaled. Use [`of`](Self::of) for unscaled input
or [`of_scaled`](Self::of_scaled) for pre-scaled values.

# Examples

```
use solverforge_core::{HardSoftDecimalScore, Score};

// Create from unscaled values (automatically multiplied by 100000)
let score1 = HardSoftDecimalScore::of(-1, -100);
assert_eq!(score1.hard_scaled(), -100000);
assert_eq!(score1.soft_scaled(), -10000000);

// Create from pre-scaled values (for minute-based penalties)
let score2 = HardSoftDecimalScore::of_scaled(-3050000, 0);  // -30.5 hard
assert!(!score2.is_feasible());

// Display shows values (trailing zeros stripped)
let score3 = HardSoftDecimalScore::of_scaled(-150000, -250000);
assert_eq!(format!("{}", score3), "-1.5hard/-2.5soft");
```
*/
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HardSoftDecimalScore {
    hard: i64,
    soft: i64,
}

impl HardSoftDecimalScore {
    /// The zero score.
    pub const ZERO: HardSoftDecimalScore = HardSoftDecimalScore { hard: 0, soft: 0 };

    /// One hard constraint penalty (scaled).
    pub const ONE_HARD: HardSoftDecimalScore = HardSoftDecimalScore {
        hard: SCALE,
        soft: 0,
    };

    /// One soft constraint penalty (scaled).
    pub const ONE_SOFT: HardSoftDecimalScore = HardSoftDecimalScore {
        hard: 0,
        soft: SCALE,
    };

    /* Creates a new score from unscaled values.

    The values are automatically multiplied by 100000.

    # Examples

    ```
    use solverforge_core::HardSoftDecimalScore;

    let score = HardSoftDecimalScore::of(-2, -100);
    assert_eq!(score.hard_scaled(), -200000);
    assert_eq!(score.soft_scaled(), -10000000);
    ```
    */
    #[inline]
    pub const fn of(hard: i64, soft: i64) -> Self {
        HardSoftDecimalScore {
            hard: hard * SCALE,
            soft: soft * SCALE,
        }
    }

    /* Creates a new score from pre-scaled values.

    Use this for minute-based penalties where precision matters.

    # Examples

    ```
    use solverforge_core::HardSoftDecimalScore;

    // -30.5 hard constraint (overlap of 30.5 minutes)
    let score = HardSoftDecimalScore::of_scaled(-3050000, 0);
    assert_eq!(score.hard_scaled(), -3050000);
    ```
    */
    #[inline]
    pub const fn of_scaled(hard: i64, soft: i64) -> Self {
        HardSoftDecimalScore { hard, soft }
    }

    #[inline]
    pub const fn of_hard(hard: i64) -> Self {
        HardSoftDecimalScore {
            hard: hard * SCALE,
            soft: 0,
        }
    }

    #[inline]
    pub const fn of_soft(soft: i64) -> Self {
        HardSoftDecimalScore {
            hard: 0,
            soft: soft * SCALE,
        }
    }

    /* Parses one level written in decimal units (`-12`, `3.25`, at most five
    fractional digits) into its scaled value, with the same rules as
    `ParseableScore::parse`.

    ```
    use solverforge_core::HardSoftDecimalScore;

    assert_eq!(HardSoftDecimalScore::parse_level("-1.5"), Ok(-150_000));
    assert!(HardSoftDecimalScore::parse_level("NaN").is_err());
    ```
    */
    pub fn parse_level(text: &str) -> Result<i64, ScoreParseError> {
        parse_fixed(text).map_err(|reason| ScoreParseError {
            message: format!("Invalid HardSoftDecimalScore level '{}': {}", text, reason),
        })
    }

    #[inline]
    pub const fn of_hard_scaled(hard: i64) -> Self {
        HardSoftDecimalScore { hard, soft: 0 }
    }

    #[inline]
    pub const fn of_soft_scaled(soft: i64) -> Self {
        HardSoftDecimalScore { hard: 0, soft }
    }

    #[inline]
    pub const fn hard_scaled(&self) -> i64 {
        self.hard
    }

    #[inline]
    pub const fn soft_scaled(&self) -> i64 {
        self.soft
    }

    pub const fn hard_score(&self) -> HardSoftDecimalScore {
        HardSoftDecimalScore::of_scaled(self.hard, 0)
    }

    pub const fn soft_score(&self) -> HardSoftDecimalScore {
        HardSoftDecimalScore::of_scaled(0, self.soft)
    }

    /* Returns true if this score has a non-zero hard component.

    Used by constraint streams to determine if a weight represents
    a hard or soft constraint.
    */
    #[inline]
    pub const fn has_hard_component(&self) -> bool {
        self.hard != 0
    }
}

impl Score for HardSoftDecimalScore {
    #[inline]
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    #[inline]
    fn zero() -> Self {
        HardSoftDecimalScore::ZERO
    }

    #[inline]
    fn levels_count() -> usize {
        2
    }

    #[inline]
    fn level_number(&self, index: usize) -> i64 {
        match index {
            0 => self.hard,
            1 => self.soft,
            _ => panic!("HardSoftDecimalScore has 2 levels, got index {}", index),
        }
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(
            levels.len(),
            2,
            "HardSoftDecimalScore requires exactly 2 levels"
        );
        HardSoftDecimalScore::of_scaled(levels[0], levels[1])
    }

    impl_score_scale!(HardSoftDecimalScore { hard, soft } => of_scaled);

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 => ScoreLevel::Hard,
            1 => ScoreLevel::Soft,
            _ => panic!("HardSoftDecimalScore has 2 levels, got index {}", index),
        }
    }

    #[inline]
    fn to_scalar(&self) -> f64 {
        self.hard as f64 * 1_000_000.0 + self.soft as f64
    }
}

impl Ord for HardSoftDecimalScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.hard.cmp(&other.hard) {
            Ordering::Equal => self.soft.cmp(&other.soft),
            other => other,
        }
    }
}

impl_score_ops!(HardSoftDecimalScore { hard, soft } => of_scaled);

impl fmt::Debug for HardSoftDecimalScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HardSoftDecimalScore({}, {})",
            Fixed(self.hard),
            Fixed(self.soft)
        )
    }
}

impl fmt::Display for HardSoftDecimalScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}hard/{}soft", Fixed(self.hard), Fixed(self.soft))
    }
}

/* One scaled level written as an exact decimal: the integer part, then up to
five fractional digits with trailing zeros trimmed. Integer arithmetic only,
so every `i64` (including `i64::MIN`) prints exactly and parses back to itself.
*/
struct Fixed(i64);

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let magnitude = self.0.unsigned_abs();
        let scale = SCALE as u64;
        let sign = if self.0 < 0 { "-" } else { "" };
        let (whole, mut fraction) = (magnitude / scale, magnitude % scale);
        if fraction == 0 {
            return write!(f, "{sign}{whole}");
        }
        let mut digits = FRACTION_DIGITS;
        while fraction % 10 == 0 {
            fraction /= 10;
            digits -= 1;
        }
        write!(f, "{sign}{whole}.{fraction:0digits$}")
    }
}

/// Number of fractional decimal digits a scaled level holds.
const FRACTION_DIGITS: usize = 5;

/* Parses one level written as `[-+]?digits(.digits)?` with at most five
fractional digits into its scaled value, rejecting anything outside `i64`
after scaling. `NaN`, infinities and exponents are not decimals and fail.
*/
fn parse_fixed(text: &str) -> Result<i64, String> {
    let (negative, unsigned) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (unsigned, None),
    };
    let is_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !is_digits(whole) || fraction.is_some_and(|fraction| !is_digits(fraction)) {
        return Err("expected a decimal number such as -12 or 3.25".to_string());
    }
    let fraction = fraction.unwrap_or("");
    if fraction.len() > FRACTION_DIGITS {
        return Err(format!(
            "at most {FRACTION_DIGITS} decimal places are representable"
        ));
    }
    let out_of_range = || "out of range for a HardSoftDecimalScore level".to_string();
    // The magnitude of `i64::MIN` exceeds `i64::MAX`, so accumulate in i128.
    let mut scaled: i128 = 0;
    for digit in whole.bytes().chain(fraction.bytes()) {
        scaled = scaled
            .checked_mul(10)
            .and_then(|value| value.checked_add(i128::from(digit - b'0')))
            .filter(|value| *value <= i128::from(i64::MAX) + 1)
            .ok_or_else(out_of_range)?;
    }
    for _ in fraction.len()..FRACTION_DIGITS {
        scaled = scaled
            .checked_mul(10)
            .filter(|value| *value <= i128::from(i64::MAX) + 1)
            .ok_or_else(out_of_range)?;
    }
    let scaled = if negative { -scaled } else { scaled };
    i64::try_from(scaled).map_err(|_| out_of_range())
}

impl ParseableScore for HardSoftDecimalScore {
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let s = s.trim();

        // Format: "0.5hard/-100.25soft" or "-1hard/0soft"
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(ScoreParseError {
                message: format!(
                    "Invalid HardSoftDecimalScore format '{}': expected 'Xhard/Ysoft'",
                    s
                ),
            });
        }

        let hard_str = parts[0]
            .trim()
            .strip_suffix("hard")
            .ok_or_else(|| ScoreParseError {
                message: format!("Hard score part '{}' must end with 'hard'", parts[0]),
            })?;

        let soft_str = parts[1]
            .trim()
            .strip_suffix("soft")
            .ok_or_else(|| ScoreParseError {
                message: format!("Soft score part '{}' must end with 'soft'", parts[1]),
            })?;

        let hard = parse_fixed(hard_str).map_err(|reason| ScoreParseError {
            message: format!("Invalid hard score '{}': {}", hard_str, reason),
        })?;
        let soft = parse_fixed(soft_str).map_err(|reason| ScoreParseError {
            message: format!("Invalid soft score '{}': {}", soft_str, reason),
        })?;

        Ok(HardSoftDecimalScore::of_scaled(hard, soft))
    }

    fn to_string_repr(&self) -> String {
        format!("{}", self)
    }
}
