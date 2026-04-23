use crate::heuristic::r#move::metadata::hash_str;

pub(crate) fn scoped_seed(
    base_seed: Option<u64>,
    descriptor_index: usize,
    variable_name: &str,
    selector_kind: &str,
) -> Option<u64> {
    base_seed.map(|seed| {
        let mixed = seed
            ^ ((descriptor_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
            ^ hash_str(variable_name).rotate_left(17)
            ^ hash_str(selector_kind).rotate_left(41);
        splitmix64(mixed)
    })
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut mixed = value;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^ (mixed >> 31)
}
