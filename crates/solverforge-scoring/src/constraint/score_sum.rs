//! Batch score accumulation with one overflow check.
//!
//! Score `Add`/`Sub`/`Neg` panic on overflow, which costs one branch per
//! operation. Scoring loops that fold many row scores into a delta instead add
//! through `Score::overflowing_*`, record whether any level wrapped, and check
//! once when the batch is finished. A wrapped level still panics, at the end of
//! the batch instead of at the row.

use solverforge_core::score::Score;
use solverforge_core::ImpactType;

pub(crate) struct ScoreSum<Sc> {
    total: Sc,
    overflow: bool,
}

impl<Sc: Score> ScoreSum<Sc> {
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            total: Sc::zero(),
            overflow: false,
        }
    }

    #[inline(always)]
    pub(crate) fn add(&mut self, score: Sc) {
        let (total, wrapped) = self.total.overflowing_add(score);
        self.total = total;
        self.overflow |= wrapped;
    }

    /// Folds in an unchecked result `(score, overflowed)` from a part.
    #[inline(always)]
    pub(crate) fn add_parts(&mut self, (score, overflowed): (Sc, bool)) {
        self.add(score);
        self.overflow |= overflowed;
    }

    #[inline(always)]
    pub(crate) fn sub(&mut self, score: Sc) {
        let (total, wrapped) = self.total.overflowing_sub(score);
        self.total = total;
        self.overflow |= wrapped;
    }

    #[inline(always)]
    #[track_caller]
    pub(crate) fn finish(self) -> Sc {
        // `black_box` keeps the flag a value: without it the optimizer turns
        // every recorded overflow into its own early branch to the panic,
        // which is the per-operation cost this type exists to avoid.
        if self.overflow {
            score_sum_overflow();
        }
        self.total
    }

    /// Returns the unchecked total and its overflow flag for a caller that
    /// folds it into a larger batch.
    #[inline(always)]
    pub(crate) fn into_parts(self) -> (Sc, bool) {
        (self.total, self.overflow)
    }

    /// `into_parts` after applying `impact` as `finish_with` does.
    #[inline(always)]
    pub(crate) fn into_parts_with(mut self, impact: ImpactType) -> (Sc, bool) {
        if impact == ImpactType::Penalty {
            let (negated, wrapped) = self.total.overflowing_neg();
            self.total = negated;
            self.overflow |= wrapped;
        }
        self.into_parts()
    }

    /// Finishes a sum of unsigned row weights and applies the constraint's
    /// impact once: a penalty negates the total, so rows are summed without a
    /// per-row sign.
    #[inline(always)]
    #[track_caller]
    pub(crate) fn finish_with(mut self, impact: ImpactType) -> Sc {
        if impact == ImpactType::Penalty {
            let (negated, wrapped) = self.total.overflowing_neg();
            self.total = negated;
            self.overflow |= wrapped;
        }
        self.finish()
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn score_sum_overflow() -> ! {
    panic!("a score delta overflows i64")
}

/// Checks an unchecked `(score, overflowed)` result: the public boundary of a
/// constraint whose internals sum without per-operation checks.
#[inline(always)]
#[track_caller]
pub(crate) fn checked<Sc>((score, overflowed): (Sc, bool)) -> Sc {
    if overflowed {
        score_sum_overflow();
    }
    score
}
