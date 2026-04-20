macro_rules! impl_bi_arity_stream {
    ($stream:ident, $builder:ident, $constraint:ident) => {
        impl_nary_arity_stream_common!(
            stream = $stream,
            builder = $builder,
            constraint = $constraint,
            filter_trait = BiFilter,
            and_filter = AndBiFilter,
            fn_filter = FnBiFilter,
            entities = [a, b],
            weight_indices = [a_idx, b_idx],
            filter_indices = [a_idx, b_idx]
        );
    };
}
