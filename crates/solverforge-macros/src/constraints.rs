mod ast;
mod emit;
mod parse;
mod plan;

#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use syn::ItemFn;

pub(crate) fn expand(input: ItemFn) -> syn::Result<TokenStream> {
    let parsed = parse::parse_constraint_function(input)?;
    let planned = plan::plan(parsed);
    emit::emit(planned)
}
