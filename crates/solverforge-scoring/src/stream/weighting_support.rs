use solverforge_core::score::{
    BendableScore, HardMediumSoftScore, HardSoftDecimalScore, HardSoftScore, Score, SoftScore,
};

#[inline]
pub(crate) fn fixed_weight_is_hard<Sc: Score>(weight: Sc) -> bool {
    Sc::levels_count() > 0 && weight.level_number(0) != 0
}

#[inline]
pub(crate) fn dynamic_weight_is_hard<Sc: Score>() -> bool {
    Sc::levels_count() == 1
        || (0..Sc::levels_count())
            .any(|index| Sc::level_label(index) == solverforge_core::score::ScoreLevel::Hard)
}

#[doc(hidden)]
pub trait ConstraintWeight<Args, Sc: Score>: Send + Sync {
    fn score(&self, args: Args) -> Sc;

    fn is_hard(&self) -> bool;
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
        dynamic_weight_is_hard::<Sc>()
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
        dynamic_weight_is_hard::<Sc>()
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
        dynamic_weight_is_hard::<Sc>()
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
        dynamic_weight_is_hard::<Sc>()
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
        dynamic_weight_is_hard::<Sc>()
    }
}
