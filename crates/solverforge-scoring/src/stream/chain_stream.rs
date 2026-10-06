/* Arbitrary-depth typed join chain, past the named arities.

`Bi`/`Tri`/`Quad`/`Penta` are ergonomic adapters with hand-written entity
accessors. Beyond Penta there is no per-arity type to write, so this is the
typed row form the arity adapters are built on: a `Chain` owns any
`Operator<S>` tree plus the next binding index, and `.join()` nests another
`JoinNode` whose plan receives the whole borrowed left row. No erasure, no
fixed ceiling — depth is whatever the type nests to.

Entry is `Penta::join`, which continues the fluent chain at depth six; a
`Chain` then joins indefinitely. Filtering wraps the tree in a `FilterNode`
(identity-preserving, view-transparent); finalizing hands the tree to the
generic `OperatorTerminal`.
*/

use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::collection_extract::CollectionExtract;
use super::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use super::relational::operator::{FilterNode, FilterPredicate, JoinNode, Operator};
use super::relational::Leaf;
use crate::constraint::relational::OperatorTerminal;

/* A typed join chain of unbounded depth.

`O` is the concrete operator tree (a `JoinNode`, optionally wrapped in a
`FilterNode`); `next_binding` is the binding index the next joined collection
receives, so repeated descriptors stay distinguishable.
*/
pub struct Chain<S: 'static, O, Sc>
where
    O: Operator<S>,
{
    tree: O,
    next_binding: u32,
    _phantom: PhantomData<fn() -> (S, Sc)>,
}

impl<S, O, Sc> Chain<S, O, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    Sc: Score + 'static,
{
    /* Wraps an existing operator tree as a chain whose next joined
    collection takes `next_binding`. */
    pub fn new(tree: O, next_binding: u32) -> Self {
        Self {
            tree,
            next_binding,
            _phantom: PhantomData,
        }
    }

    /* Adds a row filter. The filter receives the whole borrowed typed row and
    the solution, and is applied exactly once on every path. */
    pub fn filter<P>(self, predicate: P) -> Chain<S, FilterNode<O, P>, Sc>
    where
        P: for<'a> FilterPredicate<S, O::View<'a>> + 'static,
        FilterNode<O, P>: Operator<S>,
    {
        Chain {
            tree: FilterNode::new(self.tree, predicate),
            next_binding: self.next_binding,
            _phantom: PhantomData,
        }
    }

    /* Extends the chain with one more source.

    The target's condition compiles fresh, so its left key receives the whole
    borrowed row (`O::View<'a>`) and `target`'s key domain is independent of
    every earlier relationship.
    */
    pub fn join<B, EB, P>(
        self,
        target: (EB, P),
    ) -> Chain<S, JoinNode<S, O, CollectionNodeAlias<S, EB>, P::Plan>, Sc>
    where
        B: Clone + Send + Sync + 'static,
        EB: CollectionExtract<S, Item = B> + 'static,
        P: CompileCondition + 'static,
        P::Plan: IndexedPlan,
        for<'a> CollectionNodeAlias<S, EB>: Operator<S, View<'a> = Leaf<'a, B>>,
        for<'a> P::Plan: ExecutablePlan<O::View<'a>, Leaf<'a, B>>,
        JoinNode<S, O, CollectionNodeAlias<S, EB>, P::Plan>: Operator<S>,
    {
        let (extractor, condition) = target;
        let tree = JoinNode::new(
            self.tree,
            super::relational::operator::CollectionNode::new(extractor, self.next_binding),
            condition,
        );
        Chain {
            tree,
            next_binding: self.next_binding + 1,
            _phantom: PhantomData,
        }
    }

    /* Finalizes into the generic operator terminal, scoring with a weight that
    receives the whole borrowed typed row. */
    pub fn penalize<W>(self, weight: W) -> ChainBuilder<S, O, W, Sc>
    where
        W: for<'a> Fn(&S, &O::View<'a>) -> Sc + Send + Sync,
    {
        ChainBuilder {
            tree: self.tree,
            impact_type: ImpactType::Penalty,
            weight,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    /* Finalizes into the generic operator terminal as a reward. */
    pub fn reward<W>(self, weight: W) -> ChainBuilder<S, O, W, Sc>
    where
        W: for<'a> Fn(&S, &O::View<'a>) -> Sc + Send + Sync,
    {
        ChainBuilder {
            tree: self.tree,
            impact_type: ImpactType::Reward,
            weight,
            is_hard: false,
            _phantom: PhantomData,
        }
    }
}

// Alias kept short at the use site; the concrete node type.
type CollectionNodeAlias<S, E> = super::relational::operator::CollectionNode<S, E>;

/* Finalizer for a chain: holds the tree, impact, and row weight until
`named()` attaches a constraint name. */
pub struct ChainBuilder<S: 'static, O, W, Sc>
where
    O: Operator<S>,
{
    tree: O,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<fn() -> (S, Sc)>,
}

impl<S, O, W, Sc> ChainBuilder<S, O, W, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    W: for<'a> Fn(&S, &O::View<'a>) -> Sc + Send + Sync,
    Sc: Score + 'static,
    for<'a> O::View<'a>: super::relational::operator::ExplainRow,
{
    /* Marks the constraint hard. */
    pub fn hard(mut self) -> Self {
        self.is_hard = true;
        self
    }

    /* Names the constraint, producing the terminal. */
    pub fn named(self, name: &str) -> OperatorTerminal<S, O, W, Sc> {
        OperatorTerminal::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.tree,
            self.weight,
            self.is_hard,
        )
    }
}
