#[macro_export]
macro_rules! impl_incremental_penta_constraint {
    ($struct_name:ident) => {
        impl_incremental_higher_arity_constraint_common!(
            struct_name = $struct_name,
            match_kind = penta,
            entities = [a, b, c, d, e],
            match_indices = [a_idx, b_idx, c_idx, d_idx, e_idx],
            combo_positions = [pos_i, pos_j, pos_k, pos_l, pos_m],
            combo_values = [i, j, k, l, m],
            other_values = [i, j, k, l]
        );
    };
}

pub use impl_incremental_penta_constraint;
