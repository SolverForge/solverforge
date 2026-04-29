fn generate_scalar_groups_impl(model: &ModelMetadata) -> TokenStream {
    model
        .solution
        .scalar_groups_path
        .as_ref()
        .map(|path| {
            quote! {
                #path(scalar_variables)
            }
        })
        .unwrap_or_else(|| {
            quote! {
                ::std::vec::Vec::new()
            }
        })
}
