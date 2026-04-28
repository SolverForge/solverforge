use solverforge_core::score::Score;

#[inline]
pub(crate) fn fixed_weight_is_hard<Sc: Score>(weight: Sc) -> bool {
    Sc::levels_count() > 0 && weight.level_number(0) != 0
}
