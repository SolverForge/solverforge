macro_rules! impl_penta_arity_stream {
    ($stream:ident, $builder:ident, $constraint:ident) => {
        impl_nary_arity_stream_common!(
            stream = $stream,
            builder = $builder,
            constraint = $constraint,
            filter_trait = PentaFilter,
            and_filter = AndPentaFilter,
            fn_filter = FnPentaFilter,
            entities = [a, b, c, d, e],
            weight_indices = [a_idx, b_idx, c_idx, d_idx, e_idx],
            filter_indices = []
        );
    };
}
