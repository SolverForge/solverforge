#[macro_export]
macro_rules! impl_incremental_quad_constraint {
    ($struct_name:ident) => {
        impl_incremental_higher_arity_constraint_common!(
            struct_name = $struct_name,
            match_kind = quad,
            entities = [a, b, c, d],
            match_indices = [a_idx, b_idx, c_idx, d_idx],
            combo_positions = [pos_i, pos_j, pos_k, pos_l],
            combo_values = [i, j, k, l],
            other_values = [i, j, k]
        );
    };
}

pub use impl_incremental_quad_constraint;
