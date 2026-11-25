# Clojure Language Plugin

Plugin for parsing Clojure source files using `tree-sitter-clojure`.

## Grammar Notes

Clojure's homoiconic syntax means all forms are `list_lit` nodes in the
tree-sitter AST. Function definitions and branches are identified by matching
the text of the first `sym_lit` child (e.g., `defn`, `if`, `cond`).

## Queries

- `metrics.scm` — captures function definitions (`defn`/`defn-` with
  `vec_lit` parameter vector), branches, calls (any list with a symbol
  first), and assignments
- `plugin.toml` — configuration with extensions `.clj`, `.cljs`, `.cljc`,
  `.edn`, branch weights, assertion patterns, and smell weights

### Branch capture boundary

- **Single-decision conditionals** (`if`, `when`, `when-not`, `if-not`,
  `if-let`, `when-let`) capture one `@branch.if` per form.
- **`cond` / `condp` / `case`** capture per clause (`@branch.clause` /
  `@branch.switch_case`): one branch per test/result pair, pairs walking
  every second form after the head. `case` skips its dispatch expression;
  `condp` skips its predicate and value; a dangling trailing expression
  (a `case` default with no test) is not a pair. Comments inside clause
  forms are skipped.
- **`recur`** captures one `@branch.recur` (a loop back-edge).
- **`catch`** captures one `@branch.catch` per clause; `try`/`finally`
  are scaffolding and capture nothing.
- **Consumed forms** (`let`, `binding`, `do`, `try`, `finally`) claim the
  form so it never falls through to the call capture — control structure
  contributes no C-count (ABC de-inflation).
- **Logical operators** (`and`, `or`) capture one branch each.

### Nesting

Branch-bearing forms (`if`-family, `cond`/`case`/`condp`, `loop`/`for`/
`doseq`, `recur`, `catch`) wrap their body forms one level deeper as a
nested block — the adapter opts in via `[nesting] wrap_branch_bodies`,
mirroring Python's grammar-level `block` nodes. Consumed forms (`let`,
`binding`, `do`, `try`, `finally`) add no level; logical operators
(`and`/`or`) are expressions and never wrap.

### Smell weights

`[smell_weights]` (e.g. `swap!` 1.5) currently do **not** scale the ABC
C-count: `QueryEngine` never wires `WeightMaps.call` into per-call
weights, so every call contributes 1.0. This is pre-existing engine
behavior, pinned by
`clojure_control_forms_abc_semantics_pin`; revisit when the wiring lands.
Branch weights are uniform 1/1 and the manifest↔query consistency is
guarded by `every_branch_capture_has_a_manifest_entry`.

## Parameters

Single-arity `defn`/`defn-` parameter vectors report one parameter per
binding (pretender-1v8k.6):

- a bare `sym_lit` binding is one parameter, named as written (`[a b]` →
  `a`, `b`)
- `&` marks the variadic boundary and is not itself a parameter; the
  binding after it counts once, matching Python's `*rest` (`[a & rest]` →
  `a`, `rest`)
- a destructuring binding (`[b c]`, `{:keys [x]}`) counts as **one**
  parameter, named by its source text; per-element destructured counts are
  declared unsupported (no nominal-name for a binding form)
- multi-arity `defn` arity vectors are not captured yet (separate ticket)

## Predicates

Uses `#match?` (regex) predicates instead of `#eq?` to avoid duplicate
match issues seen when using separate patterns with identical structure.