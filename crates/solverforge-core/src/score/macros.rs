//! Declarative macros for reducing score type boilerplate.
//!
//! These macros generate the repetitive trait implementations that all
//! field-based score types share: arithmetic ops, ordering, multiply/divide,
//! and slash-separated parsing.

/* Generates `PartialOrd`, `Add`, `Sub`, and `Neg` for a field-based score type.

The constructor must accept fields in the order they are listed.

# Usage

`impl_score_ops!(HardSoftScore { hard, soft } => of);`
*/
macro_rules! impl_score_ops {
    ($type:ident { $($field:ident),+ } => $ctor:ident) => {
        impl PartialOrd for $type {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        // Every level computes unconditionally and one branch checks them
        // all; an overflow panics in every build profile instead of wrapping.
        impl std::ops::Add for $type {
            type Output = Self;

            #[inline]
            #[track_caller]
            fn add(self, other: Self) -> Self {
                let (sum, overflow) = $crate::score::Score::overflowing_add(self, other);
                if overflow {
                    $crate::score::arithmetic_overflow("addition");
                }
                sum
            }
        }

        impl std::ops::Sub for $type {
            type Output = Self;

            #[inline]
            #[track_caller]
            fn sub(self, other: Self) -> Self {
                let (difference, overflow) = $crate::score::Score::overflowing_sub(self, other);
                if overflow {
                    $crate::score::arithmetic_overflow("subtraction");
                }
                difference
            }
        }

        impl std::ops::Neg for $type {
            type Output = Self;

            #[inline]
            #[track_caller]
            fn neg(self) -> Self {
                let (negated, overflow) = $crate::score::Score::overflowing_neg(self);
                if overflow {
                    $crate::score::arithmetic_overflow("negation");
                }
                negated
            }
        }
    };
}

/* Generates `multiply`, `divide`, `scale`, and `abs` methods for the `Score` trait impl.

Intended to be used *inside* an `impl Score for Type { ... }` block.
Produces four method bodies that operate field-by-field.

# Usage

`impl_score_scale!(HardSoftScore { hard, soft } => of);` inside an `impl Score for T` block.
*/
macro_rules! impl_score_scale {
    ($type:ident { $($field:ident),+ } => $ctor:ident) => {
        #[inline]
        #[track_caller]
        fn multiply(&self, multiplicand: f64) -> Self {
            let (product, overflow) = self.overflowing_multiply(multiplicand);
            if overflow {
                $crate::score::float_overflow(multiplicand);
            }
            product
        }

        #[inline]
        fn overflowing_multiply(&self, multiplicand: f64) -> (Self, bool) {
            let mut overflow = !multiplicand.is_finite();
            let product = $type::$ctor( $( $crate::score::float_level(self.$field, multiplicand, &mut overflow) ),+ );
            (product, overflow)
        }

        #[inline]
        #[track_caller]
        fn divide(&self, divisor: f64) -> Self {
            let factor = 1.0 / divisor;
            let mut overflow = !divisor.is_finite() || divisor == 0.0;
            let quotient = $type::$ctor( $( $crate::score::float_level(self.$field, factor, &mut overflow) ),+ );
            if overflow {
                $crate::score::float_overflow(divisor);
            }
            quotient
        }

        #[inline]
        fn scale(&self, factor: i64) -> Self {
            // Every level multiplies unconditionally and one branch checks them all.
            let (scaled, overflow) = self.overflowing_scale(factor);
            if overflow {
                $crate::score::scale_overflow(factor);
            }
            scaled
        }

        #[inline]
        fn overflowing_scale(&self, factor: i64) -> (Self, bool) {
            let mut overflow = false;
            let scaled = $type::$ctor( $( $crate::score::track_overflow(self.$field.overflowing_mul(factor), &mut overflow) ),+ );
            (scaled, overflow)
        }

        #[inline]
        fn overflowing_add(self, other: Self) -> (Self, bool) {
            let mut overflow = false;
            let sum = $type::$ctor( $( $crate::score::add_level(self.$field, other.$field, &mut overflow) ),+ );
            (sum, overflow)
        }

        #[inline]
        fn overflowing_sub(self, other: Self) -> (Self, bool) {
            let mut overflow = false;
            let difference = $type::$ctor( $( $crate::score::sub_level(self.$field, other.$field, &mut overflow) ),+ );
            (difference, overflow)
        }

        #[inline]
        fn overflowing_neg(self) -> (Self, bool) {
            let mut overflow = false;
            let negated = $type::$ctor( $( $crate::score::neg_level(self.$field, &mut overflow) ),+ );
            (negated, overflow)
        }

        #[track_caller]
        fn abs(&self) -> Self {
            let mut overflow = false;
            let absolute = $type::$ctor( $( $crate::score::track_overflow(self.$field.overflowing_abs(), &mut overflow) ),+ );
            if overflow {
                $crate::score::arithmetic_overflow("absolute value");
            }
            absolute
        }
    };
}

/* Generates `ParseableScore` impl for scores using the `"Xsuffix/Ysuffix"` format.

Each field maps to a suffix label (e.g., `hard => "hard"`, `soft => "soft"`).
All values are parsed as `i64`.

# Usage

`impl_score_parse!(HardSoftScore { hard => "hard", soft => "soft" } => of);`
*/
macro_rules! impl_score_parse {
    ($type:ident { $($field:ident => $suffix:literal),+ } => $ctor:ident) => {
        impl $crate::score::traits::ParseableScore for $type {
            fn parse(s: &str) -> Result<Self, $crate::score::traits::ScoreParseError> {
                let s = s.trim();
                let parts: Vec<&str> = s.split('/').collect();
                let suffixes: &[&str] = &[ $($suffix),+ ];
                let count = suffixes.len();

                if parts.len() != count {
                    return Err($crate::score::traits::ScoreParseError {
                        message: format!(
                            "Invalid {} format '{}': expected {} parts separated by '/'",
                            stringify!($type), s, count
                        ),
                    });
                }

                let mut _idx = 0usize;
                $(
                    let $field = {
                        let part = parts[_idx].trim();
                        let num_str = part.strip_suffix($suffix).ok_or_else(|| {
                            $crate::score::traits::ScoreParseError {
                                message: format!(
                                    "{} part '{}' must end with '{}'",
                                    stringify!($field), part, $suffix
                                ),
                            }
                        })?;
                        let val = num_str.parse::<i64>().map_err(|e| {
                            $crate::score::traits::ScoreParseError {
                                message: format!(
                                    "Invalid {} score '{}': {}",
                                    $suffix, num_str, e
                                ),
                            }
                        })?;
                        _idx += 1;
                        val
                    };
                )+

                Ok($type::$ctor( $($field),+ ))
            }

            fn to_string_repr(&self) -> String {
                let mut parts = Vec::new();
                $(
                    parts.push(format!("{}{}", self.$field, $suffix));
                )+
                parts.join("/")
            }
        }
    };
}

// Macros are used via #[macro_use] on the module declaration.
