use quote::ToTokens;
use syn::Expr;

pub(crate) fn expression_fingerprint(expr: &Expr) -> String {
    normalize_tokens(expr.to_token_stream().to_string())
}

fn normalize_tokens(tokens: String) -> String {
    tokens.split_whitespace().collect::<Vec<_>>().join(" ")
}
