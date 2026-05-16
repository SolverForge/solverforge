use proc_macro2::TokenStream;
use syn::{Expr, Ident, ItemFn, LitStr, Stmt};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NodeId(pub(crate) usize);

#[derive(Clone, Debug)]
pub(crate) struct StreamNode {
    pub(crate) id: NodeId,
    pub(crate) binding: Ident,
    pub(crate) expression: Expr,
    pub(crate) fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ImpactKind {
    Penalty,
    Reward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TerminalKind {
    GroupedScore,
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalConstraint {
    pub(crate) stream_binding: Ident,
    pub(crate) impact: ImpactKind,
    pub(crate) weight: Expr,
    pub(crate) name: LitStr,
    pub(crate) order: usize,
    pub(crate) kind: TerminalKind,
}

#[derive(Clone, Debug)]
pub(crate) struct ConstraintFunction {
    pub(crate) item: ItemFn,
    pub(crate) prefix_statements: Vec<Stmt>,
    pub(crate) stream_nodes: Vec<StreamNode>,
    pub(crate) terminals: Vec<TerminalConstraint>,
}

#[derive(Clone, Debug)]
pub(crate) enum ConstraintProgram {
    Passthrough(ItemFn),
    SharedGrouped {
        item: ItemFn,
        prefix_statements: Vec<Stmt>,
        node: StreamNode,
        terminals: Vec<TerminalConstraint>,
    },
}

impl ImpactKind {
    pub(crate) fn helper_path(&self) -> TokenStream {
        match self {
            ImpactKind::Penalty => quote::quote! {
                ::solverforge::__internal::grouped_penalty_terminal
            },
            ImpactKind::Reward => quote::quote! {
                ::solverforge::__internal::grouped_reward_terminal
            },
        }
    }
}
