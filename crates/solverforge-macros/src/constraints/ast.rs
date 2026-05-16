use proc_macro2::TokenStream;
use syn::{Expr, Ident, ItemFn, Stmt};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ImpactKind {
    Penalty,
    Reward,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TerminalKind {
    GroupedScore,
}

#[derive(Debug)]
pub(crate) struct StreamNode {
    pub(crate) binding: String,
    pub(crate) supports_grouped_sharing: bool,
}

#[derive(Debug)]
pub(crate) struct TerminalConstraint {
    pub(crate) source_binding: String,
    pub(crate) impact: ImpactKind,
    pub(crate) weight: TokenStream,
    pub(crate) name: TokenStream,
    pub(crate) order: usize,
    pub(crate) kind: TerminalKind,
}

#[derive(Debug)]
pub(crate) enum OtherMemberKind {
    SingleConstraint,
    ConstraintSet,
}

#[derive(Debug)]
pub(crate) enum TailMember {
    Terminal(TerminalConstraint),
    Other {
        tokens: TokenStream,
        kind: OtherMemberKind,
    },
}

#[derive(Debug)]
pub(crate) struct ConstraintFunction {
    pub(crate) item: ItemFn,
    pub(crate) prefix_statements: Vec<Stmt>,
    pub(crate) tail: Option<Expr>,
    pub(crate) stream_nodes: Vec<StreamNode>,
    pub(crate) tail_members: Vec<TailMember>,
}

#[derive(Debug)]
pub(crate) struct SharedGroupedProgram {
    pub(crate) item: ItemFn,
    pub(crate) prefix_statements: Vec<Stmt>,
    pub(crate) bindings: Vec<String>,
    pub(crate) tail_members: Vec<TailMember>,
}

#[derive(Debug)]
pub(crate) enum ConstraintProgram {
    Passthrough(ConstraintFunction),
    SharedGrouped(SharedGroupedProgram),
}

impl ImpactKind {
    pub(crate) fn method_ident(&self) -> Ident {
        match self {
            ImpactKind::Penalty => Ident::new("penalize", proc_macro2::Span::call_site()),
            ImpactKind::Reward => Ident::new("reward", proc_macro2::Span::call_site()),
        }
    }
}
