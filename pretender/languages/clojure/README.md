# Clojure Language Plugin

Plugin for parsing Clojure source files using `tree-sitter-clojure`.

## Grammar Notes

Clojure's homoiconic syntax means all forms are `list_lit` nodes in the
tree-sitter AST. Function definitions and branches are identified by matching
the text of the first `sym_lit` child (e.g., `defn`, `if`, `cond`).

## Queries

- `metrics.scm` — captures function definitions (`defn`/`defn-` with
  `vec_lit` parameter vector), branches (if/when/cond/case/loop/doseq),
  calls (any list with a symbol first), and assignments
- `plugin.toml` — configuration with extensions `.clj`, `.cljs`, `.cljc`,
  `.edn`, branch weights, assertion patterns, and smell weights

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