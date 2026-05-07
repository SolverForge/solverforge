macro_rules! __solverforge_list_quote {
    ($owner_helpers:ident, $list_owner_count_terms:ident, $total_list_entities_terms:ident, $total_list_elements_terms:ident) => {
        quote! {
            #(#$owner_helpers)*

            const __SOLVERFORGE_LIST_OWNER_COUNT: usize = 0 #(+ #$list_owner_count_terms)*;

            #[inline]
            fn __solverforge_total_list_entities(s: &Self) -> usize {
                0 #(+ #$total_list_entities_terms)*
            }

            #[inline]
            fn __solverforge_total_list_elements(s: &Self) -> usize {
                0 #(+ #$total_list_elements_terms)*
            }
        }
    };
}
