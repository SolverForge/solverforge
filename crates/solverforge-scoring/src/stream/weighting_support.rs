use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SoftScore,
};

#[inline]
pub(crate) fn fixed_weight_is_hard<Sc: Score>(weight: Sc) -> bool {
    Sc::levels_count() > 0 && weight.level_number(0) != 0
}

/// Public wrapper for fixed score weights that are not one of the built-in score types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedWeight<Sc> {
    score: Sc,
}

/// Wraps a fixed score weight while preserving static dispatch.
#[inline]
pub const fn fixed_weight<Sc>(score: Sc) -> FixedWeight<Sc> {
    FixedWeight { score }
}

/// Public wrapper that marks a fixed or dynamic weight as hard metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardWeight<W> {
    weight: W,
}

/// Wraps a weight and forces hard constraint metadata while preserving static dispatch.
#[inline]
pub const fn hard_weight<W>(weight: W) -> HardWeight<W> {
    HardWeight { weight }
}

/// The weight a `penalize` or `reward` call scores each match with.
///
/// `Args` is the tuple of borrowed rows one match carries, `(&A,)` for a uni
/// stream up to `(&A, &B, &C, &D, &E)` for a penta stream; grouped streams
/// pass the key and the collector result. Implemented for:
///
/// - every built-in score type (`SoftScore`, `HardSoftScore`,
///   `HardMediumSoftScore`, `HardSoftDecimalScore`, `BendableScore`), scoring
///   each match with that fixed value;
/// - [`FixedWeight`], wrapping any other fixed score;
/// - closures `Fn(&A, ...) -> Sc` over the matched rows;
/// - [`HardWeight`], wrapping another weight and marking it hard.
///
/// Implement it for a custom weight type to compute a match's score from
/// state of your own.
///
/// # Examples
///
/// ```
/// use solverforge_core::score::SoftScore;
/// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
/// use solverforge_scoring::stream::{
///     source, ChangeSource, ConstraintFactory, ConstraintWeight,
/// };
///
/// /// Scores each late minute past a deadline.
/// struct MinutesLate {
///     deadline: i64,
/// }
///
/// impl<'a> ConstraintWeight<(&'a i64,), SoftScore> for MinutesLate {
///     fn score(&self, (end,): (&'a i64,)) -> SoftScore {
///         SoftScore::of((*end - self.deadline).max(0))
///     }
///
///     fn is_hard(&self) -> bool {
///         false
///     }
/// }
///
/// #[derive(Clone)]
/// struct Plan {
///     ends: Vec<i64>,
/// }
///
/// fn ends(plan: &Plan) -> &[i64] {
///     plan.ends.as_slice()
/// }
///
/// let constraint = ConstraintFactory::<Plan, SoftScore>::new()
///     .for_each(source(ends as fn(&Plan) -> &[i64], ChangeSource::Descriptor(0)))
///     .penalize(MinutesLate { deadline: 10 })
///     .named("late");
///
/// let plan = Plan { ends: vec![8, 13, 15] };
/// assert_eq!(constraint.evaluate(&plan), SoftScore::of(-8));
/// assert!(!constraint.is_hard());
/// ```
pub trait ConstraintWeight<Args, Sc: Score>: Send + Sync {
    /// The score one match with rows `args` contributes, before the
    /// constraint's penalty or reward sign is applied.
    fn score(&self, args: Args) -> Sc;

    /// Whether the constraint this weight scores is hard. Built-in fixed
    /// scores are hard when their first (hard) level is non-zero; closures
    /// are soft unless wrapped in [`HardWeight`].
    fn is_hard(&self) -> bool;
}

impl<'a, A, Sc: Score + Copy> ConstraintWeight<(&'a A,), Sc> for FixedWeight<Sc> {
    #[inline]
    fn score(&self, _args: (&'a A,)) -> Sc {
        self.score
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(self.score)
    }
}

impl<'a, A, B, Sc: Score + Copy> ConstraintWeight<(&'a A, &'a B), Sc> for FixedWeight<Sc> {
    #[inline]
    fn score(&self, _args: (&'a A, &'a B)) -> Sc {
        self.score
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(self.score)
    }
}

impl<'a, A, B, C, Sc: Score + Copy> ConstraintWeight<(&'a A, &'a B, &'a C), Sc>
    for FixedWeight<Sc>
{
    #[inline]
    fn score(&self, _args: (&'a A, &'a B, &'a C)) -> Sc {
        self.score
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(self.score)
    }
}

impl<'a, A, B, C, D, Sc: Score + Copy> ConstraintWeight<(&'a A, &'a B, &'a C, &'a D), Sc>
    for FixedWeight<Sc>
{
    #[inline]
    fn score(&self, _args: (&'a A, &'a B, &'a C, &'a D)) -> Sc {
        self.score
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(self.score)
    }
}

impl<'a, A, B, C, D, E, Sc: Score + Copy> ConstraintWeight<(&'a A, &'a B, &'a C, &'a D, &'a E), Sc>
    for FixedWeight<Sc>
{
    #[inline]
    fn score(&self, _args: (&'a A, &'a B, &'a C, &'a D, &'a E)) -> Sc {
        self.score
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(self.score)
    }
}

impl<'a, A, W, Sc> ConstraintWeight<(&'a A,), Sc> for HardWeight<W>
where
    W: ConstraintWeight<(&'a A,), Sc>,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A,)) -> Sc {
        self.weight.score(args)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        true
    }
}

impl<'a, A, B, W, Sc> ConstraintWeight<(&'a A, &'a B), Sc> for HardWeight<W>
where
    W: ConstraintWeight<(&'a A, &'a B), Sc>,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A, &'a B)) -> Sc {
        self.weight.score(args)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        true
    }
}

impl<'a, A, B, C, W, Sc> ConstraintWeight<(&'a A, &'a B, &'a C), Sc> for HardWeight<W>
where
    W: ConstraintWeight<(&'a A, &'a B, &'a C), Sc>,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A, &'a B, &'a C)) -> Sc {
        self.weight.score(args)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        true
    }
}

impl<'a, A, B, C, D, W, Sc> ConstraintWeight<(&'a A, &'a B, &'a C, &'a D), Sc> for HardWeight<W>
where
    W: ConstraintWeight<(&'a A, &'a B, &'a C, &'a D), Sc>,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A, &'a B, &'a C, &'a D)) -> Sc {
        self.weight.score(args)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        true
    }
}

impl<'a, A, B, C, D, E, W, Sc> ConstraintWeight<(&'a A, &'a B, &'a C, &'a D, &'a E), Sc>
    for HardWeight<W>
where
    W: ConstraintWeight<(&'a A, &'a B, &'a C, &'a D, &'a E), Sc>,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A, &'a B, &'a C, &'a D, &'a E)) -> Sc {
        self.weight.score(args)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        true
    }
}

macro_rules! impl_fixed_score_weight {
    ($score:ty) => {
        impl<'a, A> ConstraintWeight<(&'a A,), $score> for $score {
            #[inline]
            fn score(&self, _args: (&'a A,)) -> $score {
                *self
            }

            #[inline]
            fn is_hard(&self) -> bool {
                fixed_weight_is_hard(*self)
            }
        }

        impl<'a, A, B> ConstraintWeight<(&'a A, &'a B), $score> for $score {
            #[inline]
            fn score(&self, _args: (&'a A, &'a B)) -> $score {
                *self
            }

            #[inline]
            fn is_hard(&self) -> bool {
                fixed_weight_is_hard(*self)
            }
        }

        impl<'a, A, B, C> ConstraintWeight<(&'a A, &'a B, &'a C), $score> for $score {
            #[inline]
            fn score(&self, _args: (&'a A, &'a B, &'a C)) -> $score {
                *self
            }

            #[inline]
            fn is_hard(&self) -> bool {
                fixed_weight_is_hard(*self)
            }
        }

        impl<'a, A, B, C, D> ConstraintWeight<(&'a A, &'a B, &'a C, &'a D), $score> for $score {
            #[inline]
            fn score(&self, _args: (&'a A, &'a B, &'a C, &'a D)) -> $score {
                *self
            }

            #[inline]
            fn is_hard(&self) -> bool {
                fixed_weight_is_hard(*self)
            }
        }

        impl<'a, A, B, C, D, E> ConstraintWeight<(&'a A, &'a B, &'a C, &'a D, &'a E), $score>
            for $score
        {
            #[inline]
            fn score(&self, _args: (&'a A, &'a B, &'a C, &'a D, &'a E)) -> $score {
                *self
            }

            #[inline]
            fn is_hard(&self) -> bool {
                fixed_weight_is_hard(*self)
            }
        }
    };
}

impl_fixed_score_weight!(SoftScore);
impl_fixed_score_weight!(HardSoftScore);
impl_fixed_score_weight!(HardMediumSoftScore);
impl_fixed_score_weight!(HardSoftDecimalScore);

impl<'a, A, const H: usize, const S: usize> ConstraintWeight<(&'a A,), BendableScore<H, S>>
    for BendableScore<H, S>
{
    #[inline]
    fn score(&self, _args: (&'a A,)) -> BendableScore<H, S> {
        *self
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(*self)
    }
}

impl<'a, A, B, const H: usize, const S: usize> ConstraintWeight<(&'a A, &'a B), BendableScore<H, S>>
    for BendableScore<H, S>
{
    #[inline]
    fn score(&self, _args: (&'a A, &'a B)) -> BendableScore<H, S> {
        *self
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(*self)
    }
}

impl<'a, A, B, C, const H: usize, const S: usize>
    ConstraintWeight<(&'a A, &'a B, &'a C), BendableScore<H, S>> for BendableScore<H, S>
{
    #[inline]
    fn score(&self, _args: (&'a A, &'a B, &'a C)) -> BendableScore<H, S> {
        *self
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(*self)
    }
}

impl<'a, A, B, C, D, const H: usize, const S: usize>
    ConstraintWeight<(&'a A, &'a B, &'a C, &'a D), BendableScore<H, S>> for BendableScore<H, S>
{
    #[inline]
    fn score(&self, _args: (&'a A, &'a B, &'a C, &'a D)) -> BendableScore<H, S> {
        *self
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(*self)
    }
}

impl<'a, A, B, C, D, E, const H: usize, const S: usize>
    ConstraintWeight<(&'a A, &'a B, &'a C, &'a D, &'a E), BendableScore<H, S>>
    for BendableScore<H, S>
{
    #[inline]
    fn score(&self, _args: (&'a A, &'a B, &'a C, &'a D, &'a E)) -> BendableScore<H, S> {
        *self
    }

    #[inline]
    fn is_hard(&self) -> bool {
        fixed_weight_is_hard(*self)
    }
}

impl<'a, A, Sc, F> ConstraintWeight<(&'a A,), Sc> for F
where
    F: Fn(&A) -> Sc + Send + Sync,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A,)) -> Sc {
        self(args.0)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        false
    }
}

impl<'a, A, B, Sc, F> ConstraintWeight<(&'a A, &'a B), Sc> for F
where
    F: Fn(&A, &B) -> Sc + Send + Sync,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A, &'a B)) -> Sc {
        self(args.0, args.1)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        false
    }
}

impl<'a, A, B, C, Sc, F> ConstraintWeight<(&'a A, &'a B, &'a C), Sc> for F
where
    F: Fn(&A, &B, &C) -> Sc + Send + Sync,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A, &'a B, &'a C)) -> Sc {
        self(args.0, args.1, args.2)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        false
    }
}

impl<'a, A, B, C, D, Sc, F> ConstraintWeight<(&'a A, &'a B, &'a C, &'a D), Sc> for F
where
    F: Fn(&A, &B, &C, &D) -> Sc + Send + Sync,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A, &'a B, &'a C, &'a D)) -> Sc {
        self(args.0, args.1, args.2, args.3)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        false
    }
}

impl<'a, A, B, C, D, E, Sc, F> ConstraintWeight<(&'a A, &'a B, &'a C, &'a D, &'a E), Sc> for F
where
    F: Fn(&A, &B, &C, &D, &E) -> Sc + Send + Sync,
    Sc: Score,
{
    #[inline]
    fn score(&self, args: (&'a A, &'a B, &'a C, &'a D, &'a E)) -> Sc {
        self(args.0, args.1, args.2, args.3, args.4)
    }

    #[inline]
    fn is_hard(&self) -> bool {
        false
    }
}
