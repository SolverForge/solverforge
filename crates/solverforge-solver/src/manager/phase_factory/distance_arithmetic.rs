#[inline]
pub(super) fn sum_two(left: i64, right: i64) -> i64 {
    clamp_i128(i128::from(left) + i128::from(right))
}

#[inline]
pub(super) fn sum_two_minus_one(left: i64, right: i64, minus: i64) -> i64 {
    clamp_i128(i128::from(left) + i128::from(right) - i128::from(minus))
}

#[inline]
fn clamp_i128(value: i128) -> i64 {
    value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_normal_values() {
        assert_eq!(sum_two(4, 7), 11);
        assert_eq!(sum_two_minus_one(10, 8, 3), 15);
    }

    #[test]
    fn clamps_positive_overflow() {
        assert_eq!(sum_two(i64::MAX, 1), i64::MAX);
        assert_eq!(sum_two_minus_one(i64::MAX, i64::MAX, -1), i64::MAX);
    }

    #[test]
    fn clamps_negative_overflow() {
        assert_eq!(sum_two(i64::MIN, -1), i64::MIN);
        assert_eq!(sum_two_minus_one(i64::MIN, i64::MIN, 1), i64::MIN);
    }
}
