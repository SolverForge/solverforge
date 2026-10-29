/* Runtime constraint weight helpers for the weighted scoring methods.

Multipliers are applied to each constraint's aggregated contribution, never
inside per-match signing, so the unweighted methods keep their exact code.
*/

use solverforge_core::score::Score;

// Scales one constraint's contribution by its bound multiplier.
#[inline(always)]
pub(crate) fn scale_contribution<Sc: Score>(score: Sc, multiplier: u32) -> Sc {
    score.scale(i64::from(multiplier))
}

/* Scales an unchecked `(score, overflowed)` contribution, keeping it
unchecked; see `ScoreSum`. */
#[inline(always)]
pub(crate) fn scale_contribution_parts<Sc: Score>(
    (score, overflowed): (Sc, bool),
    multiplier: u32,
) -> (Sc, bool) {
    let (scaled, wrapped) = score.overflowing_scale(i64::from(multiplier));
    (scaled, overflowed | wrapped)
}

/* Splits the next `width` weight slots off `slots`.

Composite sets hand each child exactly its own slots, in structural order.
*/
#[inline]
pub(crate) fn take_slots<'m>(slots: &mut &'m [u32], width: usize) -> &'m [u32] {
    let (head, tail) = slots.split_at(width);
    *slots = tail;
    head
}
