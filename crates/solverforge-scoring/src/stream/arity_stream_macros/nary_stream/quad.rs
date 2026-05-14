macro_rules! impl_quad_arity_stream {
    ($stream:ident, $builder:ident, $constraint:ident) => {
        impl_nary_arity_stream_common!(
            stream = $stream,
            builder = $builder,
            constraint = $constraint,
            filter_trait = QuadFilter,
            and_filter = AndQuadFilter,
            fn_filter = FnQuadFilter,
            entities = [a, b, c, d],
            weight_indices = [a_idx, b_idx, c_idx, d_idx],
            filter_indices = [a_idx, b_idx, c_idx, d_idx]
        );
    };
}
