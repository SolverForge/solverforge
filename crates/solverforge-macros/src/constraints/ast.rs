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
pub(crate) enum TerminalSource {
    Binding(Ident),
    Inline {
        expression: Expr,
        fingerprint: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalConstraint {
    pub(crate) source: TerminalSource,
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
pub(crate) struct SharedGroupedProgram {
    pub(crate) item: Box<ItemFn>,
    pub(crate) prefix_statements: Vec<Stmt>,
    pub(crate) node: StreamNode,
    pub(crate) terminals: Vec<TerminalConstraint>,
    pub(crate) materialize_node: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum ConstraintProgram {
    Passthrough(Box<ItemFn>),
    SharedGrouped(Box<SharedGroupedProgram>),
}

impl ImpactKind {
    pub(crate) fn method_ident(&self) -> Ident {
        match self {
            ImpactKind::Penalty => Ident::new("penalize", proc_macro2::Span::call_site()),
            ImpactKind::Reward => Ident::new("reward", proc_macro2::Span::call_site()),
        }
    }
}
