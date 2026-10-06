macro_rules! impl_tri_arity_stream {
    ($stream:ident, $builder:ident) => {
        impl_nary_arity_stream_common!(
            stream = $stream,
            builder = $builder,
            filter_trait = TriFilter,
            and_filter = AndTriFilter,
            fn_filter = FnTriFilter,
            arity = 3,
            entities = [a, b, c],
            weight_indices = [a_idx, b_idx, c_idx],
            filter_indices = [a_idx, b_idx, c_idx]
        );
    };
}
