fn generate_scalar_groups_impl(model: &ModelMetadata) -> TokenStream {
    model
        .solution
        .scalar_groups_path
        .as_ref()
        .map(|path| {
            quote! {
                ::solverforge::__internal::bind_scalar_groups(#path(), scalar_variables)
            }
        })
        .unwrap_or_else(|| {
            quote! {
                ::std::vec::Vec::new()
            }
        })
}

fn generate_coverage_groups_impl(model: &ModelMetadata) -> TokenStream {
    model
        .solution
        .coverage_groups_path
        .as_ref()
        .map(|path| {
            quote! {
                ::solverforge::__internal::bind_coverage_groups(#path(), scalar_variables)
            }
        })
        .unwrap_or_else(|| {
            quote! {
                ::std::vec::Vec::new()
            }
        })
}
