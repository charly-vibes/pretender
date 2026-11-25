# Tasks

## 1. Clojure ABC de-inflation + weight semantics (blocks all other Clojure tasks)

- [x] 1.1 Restructure `languages/clojure/metrics.scm` so branch/assignment/definition/assertion captures precede and exclude `@call` (control forms out of the C-count); verify D6 reading of `[smell_weights]` with a unit test pinning ABC-on-fixture values; document chosen semantics in `languages/clojure/README.md`.
- [x] 1.2 Extend `try`/`catch` captures: `try` and `finally` no longer branch; one `@branch.catch` per catch clause.

## 2. Clojure capture coverage

- [ ] 2.1 Multi-arity `defn`: split into one unit per arity (engine support for vector-of-vectors parameter split); single-arity unchanged.
- [x] 2.2 Parameter extraction from arity vectors, one parameter per destructuring binding; names into the universal model.
- [x] 2.3 Per-clause `cond`/`case`/`condp` captures (`@branch.clause` per test/result pair); `recur` as `@branch.recur`.
- [ ] 2.4 Nesting model: branch/loop/recur bodies wrap as nested blocks; `let`/`binding`/`do`/`testing` do not nest.
- [ ] 2.5 `deftest` units (kind `Test`) + is-family `@assert.*` captures inside units; Method kind inside `defrecord`/`deftype`/`defprotocol` bodies.
- [x] 2.6 Complete `plugin.toml [branches]` rows for every new capture kind.

## 3. Python capture coverage

- [ ] 3.1 `match_statement` arms: one `@branch.case` per `case_clause`, `match` itself branchless.
- [ ] 3.2 Comprehension clauses: `for` clause as `@branch.loop`, each `if` filter as `@branch.comprehension-filter`.

## 4. Guardrails and tests

- [x] 4.1 Weight-table completeness guard test: every `@branch.<kind>` in a `metrics.scm` has a `[branches]` row (both adapters), fails naming the missing kind.
- [ ] 4.2 New Clojure fixtures (multi-arity defn, deftest+is-family, 3-clause cond, recur, try/catch/finally, nesting) with `;; expected_*` annotations; new `tests/clojure_roundtrip_test.rs` asserting measured values.
- [ ] 4.3 Extend `tests/python_roundtrip_test.rs` with match/comprehension fixture asserting measured values; assert-guard existing uncovered adapter forms (child .4).
- [ ] 4.4 Update both adapter READMEs: capture summary + declared-unsupported lists (Clojure: type definitions, destructured-parameter granularity, ctor kinds; Python: none new).
- [ ] 4.5 Re-adopt repo self-check ledger if ABC/count changes touch it (expected no-op — no Clojure sources in-repo); `just ci` green; SARIF snapshot byte-compat check on report_format fixtures.

## 5. Docs and ledger

- [ ] 5.1 Language README pages for any shipped-but-undocumented languages (child .13 scope: reconcile shipped set).
- [ ] 5.2 `bd close` each outcome-changing child (1v8k.2–.13) against its landed commit; epic closes when the audit table is fully green or declared-unsupported.
