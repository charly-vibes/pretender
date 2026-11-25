# Design

## Context

The two adapters feed the shared `QueryEngine` (tree-sitter query → universal model → pure metric functions). Python is complete; Clojure's `metrics.scm` under-captures (one `@branch.if` per conditional form regardless of clause count, no nesting blocks, no test units, no multi-arity split) and over-captures (catch-all `@call` on every list form). The engine already supports everything needed: branch kinds are data (`BranchKind`), weights come from `plugin.toml [branches]` via `new_with_branch_weights`, nesting/cognitive flow from block structure. No engine rewrite is required — this is adapter-query work plus small engine extensions where kinds are new.

Constraints: pinned grammars only (tree-sitter-clojure 0.1.0 is regex-`#match?`-based — no node types for defn variants beyond `list_lit`/`sym_lit`/`vec_lit`); thresholds untouched; SARIF schema-compatible; adapter shape assumed as-is (language-registry epic is separate).

## Goals / Non-Goals

**Goals:**
- Every metric in the audit table is correct for Python and Clojure, or declared unsupported in the adapter README.
- Round-trip tests pin fixture values with hand-computed annotations (existing `;; expected_complexity` convention).
- Branch-weight table completeness becomes a guarded invariant, not a convention.

**Non-Goals:**
- No threshold changes in `pretender.toml`; no new rules; no grammar bumps.
- No cross-file resolution, no new languages, no adapter restructuring (b5us epic).
- No destructuring-aware parameter granularity for Clojure (declared unsupported).

## Decisions

### D1: Adapter-first, engine-second
All capture changes land in `metrics.scm` + `plugin.toml` first; engine changes only where the model needs a new branch kind or the multi-arity unit split. The engine's `WeightMaps` path means new capture kinds need zero engine code if they reuse the generic `@branch.<kind>` matching — the kind string flows through.

### D2: Multi-arity defn = one unit per arity
A vector-of-vectors parameter node splits into one `CodeUnit` per inner vector (name `f`, params per arity). Rationale: metrics are per-unit; a merged unit would report max-based metrics across arities and break the `params_max` gate semantic. Single-arity defn is byte-identical to today.

### D3: Call-exclusion by capture precedence, not blocklist
Rather than blacklisting control symbols in the `@call` query, the adapter query restructures so branch/assignment/definition/assertion captures consume the form first; the `@call` capture matches only unconsumed leading-`sym_lit` list forms. Rationale: a blocklist rots with every new branch kind; precedence is structural. The engine's node construction order makes this a query-ordering concern, verified by the existing dedup behavior on `if` (measured: `if` is not double-counted today — extend that behavior to the whole control set).

### D4: Nesting via existing block model
Clojure nesting comes from wrapping branch/loop/recur bodies in `NestedBlock` captures — the same structure Python uses. `let`/`binding`/`do`/`testing` bodies stay in the same block (no increment). No new nesting source; `nesting_max` and cognitive weighting inherit the engine rules unchanged.

### D5: `deftest` units use kind `Test`; assertions via existing `@assert.*` path
`[assertions]` patterns in `plugin.toml` already list the is-family. The query gains a `deftest` unit capture; `is`/`are`/`testing`/`thrown?`-family forms capture `@assert.*` inside any unit. `min_assertions` then applies to Clojure test files through the existing role/threshold path with zero config change.

### D6: smell-weight semantics pinned, chosen behavior documented
Measured on the fixture today: `swap!` (weight 1.5 in `[smell_weights]`) contributes 1.0 to ABC's C-count — the weights currently feed the eval-family smell detection, not ABC. Implementation must verify this reading with a unit test and either (a) keep ABC at 1.0-per-call and document `[smell_weights]` as smell-detection-only, or (b) wire weights into ABC. Default: (a) — ABC stays a structural metric; changing ABC semantics silently would be a rule-outcome change beyond this proposal's scope. The chosen semantics is pinned by the round-trip test's expected values either way.

### D7: cond/case per-clause = one branch per test/result pair
`(cond a 1 b 2 c 3)` → 3 branches (final default expression counts only when present as a trailing pair per idiomatic `:else`). `(case e a 1 b 2 3)` → 2 clause branches + the dispatch expression is not a branch. Rationale: cyclomatic counts decision points; each clause is one.

### D8: Round-trip fixtures carry expected values in comments
New Clojure fixtures mirror the Python convention: `;; expected_complexity:`, `;; expected_params:`, `;; expected_nesting:`, `;; expected_abc:` annotations. The round-trip test parses annotations and asserts measured values — the fixture is both documentation and oracle. The audit's hand-computed table in proposal.md is the source for those annotations.

## Risks / Trade-offs

- [Regex-based Clojure query mis-capturing on nested forms] → each capture change lands with a fixture that exercises it; round-trip test fails loudly.
- [Grammar ceiling: 0.1.0 can't see docstrings/maps distinctly] → declared-unsupported list absorbs; no grammar bump in scope.
- [ABC re-baselining for Clojure users] → ABC values drop where control forms were inflating them; that is the fix, not a regression. Ledger users re-adopt once (`--adopt`), documented in the change tasks.
- [swapping branch kinds breaks weight lookups] → weight-table guard test fails at commit time (D-invariant), not at user runtime.

## Migration Plan

1. Land adapter + engine changes per child tickets, each with fixtures green.
2. Re-adopt the repo's own ledger once (`just dogfood` must stay green; pretender's self-check has no Clojure sources, so no-op expected — verified in tasks).
3. Merge via the epic's PR; `gate_baseline_ids_are_a_subset_of_origin_main` resolves at merge as usual.

## Open Questions

- None blocking. D6's abc-vs-smell-weights reading is verified by the first Clojure implementation task before any downstream child lands.
