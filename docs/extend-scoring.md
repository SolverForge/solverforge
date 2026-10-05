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

`candidate_matches` may omit only predicates already guaranteed by candidate
selection. `matches` remains the complete relationship. Preserve authored
short-circuit order for residual conditions. Key and predicate functions must
be pure with respect to notified data.

Full evaluation builds fresh indexes. Use `insert_right_transient` when rows
will never retract: there is no need to retain reverse old keys in that case.
Persistent initialization must retain reverse keys, because retraction removes
old entries before changed keys are extracted.

## Notifications and terminal scoring

Route each descriptor notification through every applicable binding. A binary
join retracts affected outputs once, updates both input branches before insertion
probing, and deduplicates outputs by joined identity. Repeated insertion and
retraction are no-ops. Static and unknown sources retain `ChangeSource` semantics;
unknown changes must not silently trigger a rebuild fallback.

`OperatorTerminal` retains each signed contribution in a checked generational
side-table. Return the inverse retained score on retraction; do not recompute
it from changed entities. Accumulate callbacks as `retract_delta + insert_delta`.
Full evaluation and match counting are meaningful before initialization.
Explanation payload conversion is confined to the existing cold `EntityRef`
boundary.

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

Both `retract` and `insert` return `RowChanges` with removed and inserted handles.
Removing a contributor from a surviving group replaces its old aggregate row
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

`ExistenceNode` uses the same compiled condition plans for semi/anti joins over
arbitrary producers. It retains matching links/counts, not scored pair rows.
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

## Verify extensions

Compare full and retained row multisets with an independent nested-loop oracle.
Test both mutation sides, duplicate keys, repeated descriptors, residual-only
changes, stale handles, repeated callbacks, unrelated descriptors, reset, and
reinitialization. Keep exact collector retraction tokens when adding grouped
ownership. Run scoring tests, strict clippy, and workspace integration tests.
Measure release wall time with equivalent match counts; allocation and
instruction measurements support, but do not replace, throughput evidence.
