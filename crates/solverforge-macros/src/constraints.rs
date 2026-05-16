mod ast;
mod emit;
mod fingerprint;
mod normalize;
mod parse;
mod plan;

#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use syn::ItemFn;

pub(crate) fn expand(input: ItemFn) -> syn::Result<TokenStream> {
    let parsed = parse::parse_constraint_function(input)?;
    let normalized = normalize::normalize(parsed);
    let planned = plan::plan(normalized);
    emit::emit(planned)
}
