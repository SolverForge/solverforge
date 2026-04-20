#[macro_export]
macro_rules! impl_incremental_tri_constraint {
    ($struct_name:ident) => {
        impl_incremental_higher_arity_constraint_common!(
            struct_name = $struct_name,
            match_kind = tri,
            entities = [a, b, c],
            match_indices = [a_idx, b_idx, c_idx],
            combo_positions = [pos_i, pos_j, pos_k],
            combo_values = [i, j, k],
            other_values = [i, j]
        );
    };
}

pub use impl_incremental_tri_constraint;
