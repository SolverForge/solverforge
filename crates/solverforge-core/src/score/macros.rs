//! Declarative macros for reducing score type boilerplate.
//!
//! These macros generate the repetitive trait implementations that all
//! field-based score types share: arithmetic ops, ordering, multiply/divide,
//! and slash-separated parsing.

/// Generates `PartialOrd`, `Add`, `Sub`, and `Neg` for a field-based score type.
///
/// The constructor must accept fields in the order they are listed.
///
/// # Usage
/// ```ignore
/// impl_score_ops!(HardSoftScore { hard, soft } => of);
/// impl_score_ops!(HardSoftDecimalScore { hard, soft } => of_scaled);
/// ```
macro_rules! impl_score_ops {
    ($type:ident { $($field:ident),+ } => $ctor:ident) => {
        impl PartialOrd for $type {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl std::ops::Add for $type {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                $type::$ctor( $(self.$field + other.$field),+ )
            }
        }

        impl std::ops::Sub for $type {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                $type::$ctor( $(self.$field - other.$field),+ )
            }
        }

        impl std::ops::Neg for $type {
            type Output = Self;

            fn neg(self) -> Self {
                $type::$ctor( $(-self.$field),+ )
            }
        }
    };
}

/// Generates `multiply`, `divide`, and `abs` methods for the `Score` trait impl.
///
/// Intended to be used *inside* an `impl Score for Type { ... }` block.
/// Produces three method bodies that operate field-by-field.
///
/// # Usage
/// ```ignore
/// impl Score for HardSoftScore {
///     // ...other methods...
///     impl_score_scale!(HardSoftScore { hard, soft } => of);
/// }
/// ```
macro_rules! impl_score_scale {
    ($type:ident { $($field:ident),+ } => $ctor:ident) => {
        fn multiply(&self, multiplicand: f64) -> Self {
            $type::$ctor( $( (self.$field as f64 * multiplicand).round() as i64 ),+ )
        }

        fn divide(&self, divisor: f64) -> Self {
            $type::$ctor( $( (self.$field as f64 / divisor).round() as i64 ),+ )
        }

        fn abs(&self) -> Self {
            $type::$ctor( $( self.$field.abs() ),+ )
        }
    };
}

/// Generates `ParseableScore` impl for scores using the `"Xsuffix/Ysuffix"` format.
///
/// Each field maps to a suffix label (e.g., `hard => "hard"`, `soft => "soft"`).
/// All values are parsed as `i64`.
///
/// # Usage
/// ```ignore
/// impl_score_parse!(HardSoftScore { hard => "hard", soft => "soft" } => of);
/// impl_score_parse!(HardMediumSoftScore { hard => "hard", medium => "medium", soft => "soft" } => of);
/// ```
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
