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
join target. Join either branch with another operator;
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

## Current ownership boundary

The current full-traversal signature ties emitted views to the operator/solution
borrow. Collection and joined borrowed rows satisfy that contract. It does not
yet supply an evaluation-state owner for newly computed non-Clone projected or
grouped values. Do not implement such producers by leaking temporaries, cloning
collector results, caching solution references, or reusing a stale initialized
result. Owned derived producers and compiler-owned derived-consumer sharing
require further protocol integration; they are not supplied by `JoinNode` alone.

## Verify extensions

Compare full and retained row multisets with an independent nested-loop oracle.
Test both mutation sides, duplicate keys, repeated descriptors, residual-only
changes, stale handles, repeated callbacks, unrelated descriptors, reset, and
reinitialization. Keep exact collector retraction tokens when adding grouped
ownership. Run scoring tests, strict clippy, and workspace integration tests.
Measure release wall time with equivalent match counts; allocation and
instruction measurements support, but do not replace, throughput evidence.
