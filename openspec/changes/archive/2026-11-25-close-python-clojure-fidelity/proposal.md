# Proposal: Close Python and Clojure analyzer fidelity gaps

## Why

A measured audit of the two adapters (fixtures + edge-case files, binary at clean HEAD) found Clojure's metric surface flat, dead, or inflated, and two Python blind spots that under-report complexity:

| metric | Python | Clojure | consequence |
|---|---|---|---|
| params | works | **always 0** | `params_max` can never fire for Clojure |
| nesting_max | works | **always 0** | `nesting_max` can never fire for Clojure |
| assertions | works | **always 0** | `min_assertions` is vacuous for Clojure test files |
| abc | works | **inflated** | control forms enter the C-count via the catch-all `@call` capture |
| multi-arity defn | n/a | **not captured** | the function is invisible to every metric |
| cond/case | n/a | **1 branch per form** | a 3-clause `cond` counts like an `if` |
| try/catch | works | **double-counted** | `try` and `catch` both tagged `@branch.catch` |
| type definitions | Class + fields | none (by design) | data-model checks silently absent |
| unit kind | Function/Method/Ctor | Function only | no Method/Ctor distinction |

The gap is worth closing now: the structural gate's promise ("new violations fail CI") is only as strong as the weakest shipped adapter, and Clojure currently cannot produce half its metric surface.

This proposal is filed first per project rule — every child of pretender-1v8k changes what the reported metrics are, and *what is measured* changes require an openspec proposal. Thresholds in `pretender.toml` stay untouched; this change alters measurement, not violation firing.

## What Changes

### Python (tree-sitter-python, pinned grammar)
1. **match/case arms are branches** — each `case_clause` captures `@branch.case` (cyclomatic 1, cognitive 1, nesting-weighted like the existing branch set). A `match_statement` itself adds no cyclomatic count.
2. **Comprehension filters are branches** — each `if` clause inside a comprehension captures `@branch.comprehension_filter` (cyclomatic 1, cognitive 1). The comprehension's `for` clause captures `@branch.loop` (same weights as `for` statements).

### Clojure (tree-sitter-clojure 0.1.0, pinned grammar)
3. **Multi-arity defn captured** — `(defn name (params body)+)` with a vector-of-vectors produces one unit per arity, named `name` with `params` from that arity's vector; single-arity defn unchanged.
4. **Parameters extracted** — the arity vector's symbol elements become unit parameters (same as Python). Destructuring forms (`[a [b c]]`, `{:keys [x]}`) count as **one parameter** (the binding), not per inner symbol — deep destructuring is declared unsupported (see below).
5. **Control forms stop being calls** — the catch-all `@call` capture excludes any form already captured as a branch, assignment, function definition, or assertion. This is the ABC de-inflation fix.
6. **Per-clause cond/case** — `cond`/`case`/`condp` capture one `@branch.clause` per test/result pair (cyclomatic 1, cognitive 1 each), not one `@branch.if` per form.
7. **`recur` is a branch** — captures `@branch.recur` (cyclomatic 1, cognitive 1).
8. **try/catch single-count** — `try` no longer captures a branch; only each `catch` clause captures `@branch.catch`. `finally` is neither branch nor call.
9. **Nesting model** — nesting depth increments for forms nested inside a branch/loop body within the same unit (same model the Python adapter already feeds `nesting_max` and cognitive weighting through the shared engine). `let`/`binding`/`do` bodies do not increment nesting.
10. **deftest forms are units** — `(deftest name ...)` captures a unit with kind `Test`, and the is-family assertions inside it (`is`, `are`, `testing`, `thrown?`, `thrown-with-msg?`, `assert`, `assert-predicate`) count through the existing `assertions` metric. `testing` contexts nest without incrementing nesting.
11. **Branch-weight table completed** — every capture in `metrics.scm` has an entry in `plugin.toml [branches]`; a test guards the invariant "no capture kind without a weight row".
12. **Unit kind resolution** — `defn` inside a `defrecord`/`deftype`/`defprotocol` protocol or impl body resolves kind `Method`; top-level `defn` stays `Function`. Clojure has no constructors (declared unsupported).

### Both languages
13. **Declared-unsupported list** — written into `languages/<lang>/README.md` so silence is intentional: Clojure — type definitions (no nominal types), per-element destructured parameters, constructor kind; Python — none new (decorated/async forms already covered).
14. **ABC smell-weight semantics pinned by test** — measured behavior today: `[smell_weights]` entries (e.g. `swap!` at 1.5) do not visibly change ABC on the fixture; implementation must either apply them to the C-count or scope `[smell_weights]` to the eval-family smell detection only, with a unit test pinning the chosen semantics.

## Capabilities

### Modified Capabilities
- `universal-code-model`: capture sets per language (branch kinds, parameters, unit kinds, assertions, nesting), the Clojure call-capture exclusion rule, multi-arity defn modeling, the completed branch-weight table invariant, and the declared-unsupported list. Assertion counting flows through the existing query-contract requirement (Clojure's is-family patterns already ship in `plugin.toml [assertions]`); no threshold schema changes.

## Impact

- Affected specs: `universal-code-model` (MODIFIED: Branch Model, Tree-Sitter Query Contract, Code Unit Model, Parameter Extraction, Call Site Weighting; ADDED: Clojure Nesting Model, Branch Weight Table Completeness, Declared Unsupported Metrics).
- Affected code: `pretender/languages/python/metrics.scm` + `plugin.toml`, `pretender/languages/clojure/metrics.scm` + `plugin.toml` + `README.md`, `pretender/languages/python/README.md`, `pretender/src/engine/` (branch-kind handling for the new captures), `pretender/src/metric_application/`, `pretender/tests/python_roundtrip_test.rs`, new `pretender/tests/clojure_roundtrip_test.rs` + fixtures.
- No new grammar dependency: tree-sitter-python and tree-sitter-clojure stay pinned.
- SARIF: no rule IDs added or removed; findings only change in *when they fire* (metrics become real), schema stays compatible.
- Out of scope: the language-registry epic (pretender-b5us) — adapter shape is assumed as-is; thresholds in `pretender.toml`.

## Metric deltas on named fixtures

Measured at HEAD (binary, neutral directory), hand-computed targets per the rules above:

`tests/fixtures/clojure_sample.clj`:
| unit | metric | before (measured) | after (expected) |
|---|---|---|---|
| simple | params / abc | 0 / 1.0 | 1 / 1.0 (`inc` stays a call) |
| with-branch | params / abc | 0 / 2.24 | 1 / 2.24 (`pos?`, `-` stay calls) |
| complex-func | params | 0 | 3 (`a b items`) |
| complex-func | nesting_max | 0 | 2 (if → if, doseq → if) |
| complex-func | abc | 8.06 (control forms in C-count) | recomputed with control forms excluded; exact value pinned by the round-trip test |
| complex-func | cyclomatic | 5 (matches fixture annotation) | 5 (unchanged — branch set already complete here) |

`tests/fixtures/python_cognitive.py` / new `match`/comprehension fixture:
| unit | metric | before | after |
|---|---|---|---|
| function with 3-arm match | cyclomatic | 1 (arms invisible) | 3 |
| comprehension with 2 filters | cyclomatic | 1 (filters invisible) | 2 (loop + filters) |

New Clojure fixtures (added by the round-trip child): multi-arity defn, deftest with is-family, 3-clause cond, recur in loop, try/catch/finally — each with hand-computed expected values in the test.

## Acceptance

- Every metric in the finding table produces a correct value for Python and Clojure, or is declared unsupported in the language README.
- All round-trip tests (existing Python, new Clojure) pin before/after fixture values; hand-computed annotations live in fixture comments like the existing `;; expected_complexity` convention.
- `just ci` green; SARIF schema-compatible; `pretender.toml` untouched.
