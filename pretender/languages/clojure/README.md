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

## Predicates

Uses `#match?` (regex) predicates instead of `#eq?` to avoid duplicate
match issues seen when using separate patterns with identical structure.