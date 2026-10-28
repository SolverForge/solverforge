/* Runtime constraint weight helpers for the weighted scoring methods.

Multipliers are applied to each constraint's aggregated contribution, never
inside per-match signing, so the unweighted methods keep their exact code.
*/

use solverforge_core::score::Score;

// Scales one constraint's contribution by its bound multiplier.
#[inline]
pub(crate) fn scale_contribution<Sc: Score>(score: Sc, multiplier: u32) -> Sc {
    if multiplier == 1 {
        score
    } else {
        score.scale(i64::from(multiplier))
    }
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
