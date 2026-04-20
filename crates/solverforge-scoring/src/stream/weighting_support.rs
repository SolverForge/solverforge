use solverforge_core::score::Score;

#[inline]
pub(crate) fn fixed_weight_is_hard<Sc: Score>(weight: Sc) -> bool {
    weight
        .to_level_numbers()
        .first()
        .map(|&hard| hard != 0)
        .unwrap_or(false)
}
