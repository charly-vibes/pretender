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
- **Consumed forms** (`let`, `binding`, `do`, `try`, `finally`, `testing`)
  claim the form so it never falls through to the call capture — control
  structure contributes no C-count (ABC de-inflation). `testing` inside
  `deftest` bodies is scaffolding too: it adds no call and no branch,
  while assertions nested inside it still count.
- **Logical operators** (`and`, `or`) capture one branch each.

### Unit kinds

- **Function**: `defn`/`defn-` (single arity, per the parameters section).
- **Test**: `deftest` forms are units of kind `Test` (pretender-1v8k.8).
  A deftest has no parameter vector, so it has its own definition query.
  A `defn` nested inside a `deftest` body stays `Function` — `deftest`
  is not a type scope.
- **Method**: a `defn` inside a `defrecord`/`deftype`/`defprotocol`
  body resolves to `Method` — the enclosing head symbol decides the
  kind, mirroring Python's `class_definition` scoping. The
  `defrecord`/`deftype` form itself is not a unit (zero type captures,
  see the grammar notes).

### Assertions

`is`, `is-not`, `are`, `assert`, `thrown?`, `thrown-with-msg?`, and
`with-test` forms capture `@assert.statement` and count toward a unit's
assertion total; the `[assertions]` patterns in `plugin.toml` mirror the
same is-family set for rule scopes.

### Nesting

Branch-bearing forms (`if`-family, `cond`/`case`/`condp`, `loop`/`for`/
`doseq`, `recur`, `catch`) wrap their body forms one level deeper as a
nested block — the adapter opts in via `[nesting] wrap_branch_bodies`,
mirroring Python's grammar-level `block` nodes. Consumed forms (`let`,
`binding`, `do`, `try`, `finally`) add no level; logical operators
(`and`/`or`) are expressions and never wrap.

### Smell weights

`[smell_weights]` (e.g. `swap!` 1.5) scale the ABC C-count per call:
`QueryEngine::new_with_branch_weights` wires `WeightMaps.call` into
per-call weights, and each call contributes its declared weight
(defaulting to 1.0 for callees with no entry) in place of a bare call.
The wired semantics are pinned by
`clojure_control_forms_abc_semantics_pin`; branch weights are uniform
1/1 and the manifest↔query consistency is
guarded by `every_branch_capture_has_a_manifest_entry`.

## Parameters

Multi-arity `defn`/`defn-` (`(defn f ([x] x) ([x y] …))`) becomes **one
unit per arity clause** (pretender-1v8k.5): every clause yields a unit
named `f` (no arity suffix), whose parameters, complexity, and
function_lines are computed from that clause only; the clause list node
scopes the unit's span. Single-arity `defn`/`defn-` behave exactly as
before.

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

## Predicates

Uses `#match?` (regex) predicates instead of `#eq?` to avoid duplicate
match issues seen when using separate patterns with identical structure.

## Known unsupported metrics

- **Type definitions and fields** — *Not supported by the grammar*:
  tree-sitter-clojure 0.1 exposes only generic `list_lit`/`sym_lit` nodes,
  so `defrecord`/`deftype`/`defprotocol` bodies cannot be captured
  distinctly; zero `@type.*` captures are deliberate (archived design
  D2/D3).
- **ABC call weights** — *Implemented (pretender-x078)*: `[smell_weights]`
  reach the engine's per-call weights; every call contributes its
  declared weight (default 1.0). See Smell weights above.