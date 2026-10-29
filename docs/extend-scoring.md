# Extending typed relational scoring

The low-level relational surface is in
`solverforge_scoring::stream::relational::operator`; its scoring terminal is
`solverforge_scoring::constraint::relational::OperatorTerminal`. These are
concrete generic types, not an erased runtime graph. The existing fluent
stream families have not all migrated to this protocol.

## Borrowed rows and identities

`Operator<S>` supplies a `Copy` borrowed `View<'a>`, full traversal, initialization,
handle enumeration and resolution, provenance traversal, and descriptor-local
retract/insert deltas. Copy applies to the view, not its entities.
`CollectionNode` produces `Leaf<'a, A>`; `JoinNode` produces recursive
`Pair<L::View<'a>, R::View<'a>>`. `FilterNode` preserves accepted input
identities and checks membership at its own boundary, including on a right
join target. `MergeNode` unions compatible row views without collapsing equal
values across branches. Join either branch with another operator;
there is no internal binding-count limit. Public named arity adapters remain
separate from this low-level row form.

For the first unary keyed equality cross-join, the left unary filter becomes
source membership before key extraction and pair creation; a filtered unary
right target supplies its own membership. Original source indexes and change
metadata are preserved across incremental membership transitions. Post-join
filters operate on joined rows. This left-filter pushdown does not apply to
comparison, overlap, composed-condition, predicate, or same-source self-join
dispatch. The membership adapter is hidden, not a public modeling API.

Resolve source entities against the current solution. Never retain solution
references across notifications. Output identities refer to generational input
handles, not equality keys, entity values, or source indexes. Preserve every
binding in provenance, including repeated physical entities. Storage slots
are not semantic filter indexes.

## Compile conditions, then execute their indexes

`CompileCondition` selects a concrete plan. `IndexedPlan` owns its index type
and structural kind, independently of borrowed view lifetimes.
`ExecutablePlan<L, R>` extracts input keys, inserts/removes handles, probes
candidate handles, and checks residual predicates.

Equality probes borrow exact hash buckets: hash collisions still go through
`Eq`. Equality conjunctions compile to one heterogeneous nested tuple key.
Equality components separated by predicates or ranges normalize into the same
composite hash relationship; only the non-equality conditions remain residuals.
Comparison plans use ordered buckets; overlap plans narrow candidates and
apply the exact interval predicate. Arbitrary predicate plans explicitly scan
retained opposite rows. Never replace a predicate scan with a fabricated
constant equality key.

Conditions may be authored over entities (`Fn(&A) -> T`) while the operator
tree hands plans leaf views. Convert at the boundary with a named plan that
stores the entity closures and reads `.entity` off each leaf: `BiUnaryPlan`
for equality, `ViewComparisonPlan`/`ViewOverlapPlan` for ordered and interval
conditions. `ToViewPlan` performs that conversion for composed conjunctions
and keeps the equality path unchanged, so downstream grouping, projection, and
flattening keep their `into_keys`. A mixed equality+residual conjunction
compiles to `EqualityWithResidual`, which is itself a compiled condition and
composes into a single stream relationship.

`candidate_matches` may omit only predicates already guaranteed by candidate
selection. `matches` remains the complete relationship. Preserve authored
short-circuit order for residual conditions. Key and predicate functions must
be pure with respect to notified data.

Full evaluation builds fresh indexes. Use `insert_right_transient` when rows
will never retract: there is no need to retain reverse old keys in that case.
Persistent initialization must retain reverse keys, because retraction removes
old entries before changed keys are extracted.

Join depth is recursive, not per-arity. `Bi`/`Tri`/`Quad`/`Penta` are fluent
cross-stream adapters over the same tree; `stream::cross::Penta::join` returns an arity-free
`stream::chain::Chain`, and `Chain::join` nests another `JoinNode` whose plan's
left view is the whole borrowed prior row. Use named `fn` items with explicit
lifetimes for keys over nested pair rows — a closure annotated over a nested
row fixes the borrow lifetime and fails the higher-ranked `ExecutablePlan`
bound. `Chain::filter` wraps the tree in an identity-preserving `FilterNode`.
Every filter authored on the cross `Bi`, `Tri`, `Quad` or `Penta` stream is
applied once by `PentaScored` at the penta boundary, and `Penta::join` builds
the `Chain` over that filtered producer. Same-source `PentaConstraintStream` is
terminal. Fluent right-hand targets implement `CollectionExtract`, including
filtered unary streams; arbitrary derived producers are inputs to low-level
`JoinNode`, not interchangeable fluent targets.

## Notifications and terminal scoring

Route each descriptor notification through every applicable binding. A binary
join retracts affected outputs once, updates both input branches before insertion
probing, and deduplicates outputs by joined identity. Repeated insertion and
retraction are no-ops. A `Static` source ignores every descriptor notification, and a
`Descriptor(idx)` source reacts only to its own descriptor; there is no
unlocalized source and no rebuild fallback.

`OperatorTerminal` retains each signed contribution in a checked generational
side-table. Return the inverse retained score on retraction; do not recompute
it from changed entities. Accumulate callbacks as `retract_delta + insert_delta`.
Full evaluation and match counting are meaningful before initialization.
Explanation payload conversion is confined to the existing cold `EntityRef`
boundary.

Runtime constraint weights never touch that signing step. A score director's
weight mode is a type (`ScoreDirector<S, C, Desc, M>`, `M` = `UnweightedMode`
or `WeightedMode`) fixed when it is built, so an unweighted director runs the
unweighted methods unchanged and never names a weighted one. A weighted
director calls the `*_weighted`
variants with one multiplier per weight slot, and every multiplier scales a
constraint's whole contribution: its total, its incremental delta, and its
match scores. Integer scaling is linear, so this equals scaling every match,
and retained state stays unweighted. A new terminal that implements
`IncrementalConstraint` for one named constraint inherits correct
`*_weighted` defaults. A type that scores several named constraints behind one
`ConstraintSet` (like a shared terminal's `ScorerChain`) must expose one weight
slot per named constraint through `constraint_weight_refs` and scale each
one's contribution by its own slot; never apply one multiplier to a delta that
sums several named constraints. A hand-written `ConstraintSet` must implement
the weighted methods by splitting `multipliers` across its children in
`constraint_weight_refs` order.

## Evaluation ownership

`prepare_evaluation` creates a concrete `Evaluation` owner independent of retained
incremental state. `visit_evaluation` borrows that owner for the duration of a
traversal. The root `visit_all` accepts a higher-ranked visitor, so newly computed
owned values cannot escape their local owner. Collection evaluation state is
unit; joins, filters, and unions compose child owners without materializing root
output rows. A join still streams its left input while indexing borrowed right
views.

`ProjectNode` owns zero or more mapped values per input identity. Its full
evaluation owner stores fresh emissions, and its retained store owns incremental
emissions separately. `ProjectView` carries the borrowed value, emission position,
and complete upstream view. The projection mapper is called once per input in
each full evaluation; upstream traversal is replayed to attach borrowed input
views without self-referential storage. Callbacks must be pure and deterministic,
including their emission ordering. Projected payloads need not be Clone, Copy,
Debug, or PartialEq. Explanations traverse original contributors rather than
cloning produced payloads.

Never leak temporaries, cache solution references, or reuse stale initialized
results for recomputation.

## Group results and replacement deltas

`GroupNode` consumes arbitrary row producers and retains exactly the token returned
by each owned accumulation. Group results are producers, not finalized scores.
`GroupView::with_result` borrows the accumulator's result without a Clone bound;
its scope also supports collectors that compute a temporary result view. The
view retains a borrowed lineage owner rather than copying contributor payloads
into downstream rows. Cold explanations resolve every contributing input row,
including repeated bindings in joined input.

`retract` and `insert` write into caller-owned `removed` / `inserted`
`Vec<RowHandle>` buffers, which every operator clears on entry; one event
yields one removed/inserted pair. Removing a contributor from a surviving group replaces its old aggregate row
immediately. Returning only removed handles on retraction is incorrect: it loses
changed nonempty groups, and a semi/anti join can also gain membership during
retraction. Consumers remove all old indexes/scores before inserting replacement
rows. Source retractions still have no inserts; their behavior is unchanged.
A group coalesces every affected contributor in one notification before publishing
one final replacement per changed group. Old keys, scores, and collector tokens
are retained independently, so downstream invalidation does not recompute an old
value from a mutated accumulator.

Compiler-owned derived-consumer sharing remains separate work; it is not supplied
by a projection, group, or join alone.

Existence has two operators. `ExistenceNode` is the pure-key semi/anti join: it
takes key extractors rather than a condition plan and keeps per-key counts, not
pair rows. `JoinerExistenceNode` compiles a condition plan with a residual
predicate and retains per-pair links. Neither retains scored pair rows.
Only 0↔nonzero counts change left membership; right updates that keep a match
present do not rescore an unchanged left row. Output lineage contains left
bindings only, as the right relation is a membership test rather than a new
binding. Grouped right replacements are processed atomically, including during
contributor retraction.

`FlattenNode` borrows child slices from arbitrary input rows and retains only
the parent handle and child position. `FlattenView` carries the complete parent
view, so source semantic indexes and all contributing bindings stay available.
The extractor takes an explicitly lifetime-bound solution reference as well as
the borrowed input view; this makes lending function items legal despite GAT
input lifetime elision in Rust's higher-ranked `Fn` output bounds. Full evaluation
streams child rows and borrows any projected parent from its evaluation owner.
Duplicate children are distinct positions, not deduplicated values.

`ComplementNode` owns one indexed membership relation containing its target and
result producers. It publishes matched target/result pairs and owned defaults
for unmatched targets. Only the target domain produces outputs: a real group
without a matching target does not fabricate a complemented row. Duplicate
target keys remain separate target identities and receive separate contributions.
Default↔real replacements remove the old output before publishing the new one;
real grouped results borrow their existing accumulator rather than copying it.
The enum borrowed view remains usable by downstream conditions and projections.

## Custom `IncrementalConstraint`

A constraint the streams cannot express implements `IncrementalConstraint`
by hand, with an empty `SingleConstraint` impl for the same type beside
it. The marker tells the compiler the type is one constraint, so it composes
in a tuple next to fluent constraints. Never implement the marker for a tuple,
chain or other multi-constraint type. The score director drives an
implementation through this contract:

- `initialize` runs on a fresh or reset constraint before any notification
  and returns the same score as `evaluate` on that solution.
- For each change to entity `entity_index` of entity class `descriptor_index`,
  `on_retract` runs once with the solution still in its old state and returns
  the negation of the score the entity's matches contribute; `on_insert` runs
  once after the change and returns the score they now add. `initialize` plus
  every delta since must equal `evaluate` on the current solution.
- Entity indexes are local to their entity class. A notification for a class
  the constraint does not read returns zero.
- `reset` drops all retained state, and `initialize` runs again before the
  next notification.
- `constraint_ref` stays the same for the constraint's life, and `is_hard`
  agrees with every other constraint sharing its full reference; building the
  set's metadata panics on conflicting hardness.

`crates/solverforge/tests/custom_constraint.rs` is a worked example: a
per-resource load penalty that retains each resource's load, composed with a
fluent constraint and checked against a from-scratch score after every
change.

## Verify extensions

Compare full and retained row multisets with an independent nested-loop oracle.
Test both mutation sides, duplicate keys, repeated descriptors, residual-only
changes, stale handles, repeated callbacks, unrelated descriptors, reset, and
reinitialization. Keep exact collector retraction tokens when adding grouped
ownership. Run scoring tests, strict clippy, and workspace integration tests.
Measure release wall time with equivalent match counts; allocation and
instruction measurements support, but do not replace, throughput evidence.
