# R

Reference for which tree-sitter nodes pretender tracks for R files, how
each contributes to `pretender check` metrics, and known measurement gaps.
Consult this page when tuning thresholds or diagnosing unexpected scores.

## Overview

Analyses `.r`/`.R` files using a tree-sitter grammar. R functions are
anonymous; pretender attaches the name from the left-hand side of the
`<-`, `=`, or `<<-` assignment that binds them. Branch coverage covers
`if`, the four loop forms, and short-circuit operators.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `binary_operator` `<-` / `=` / `<<-` with `function_definition` rhs | `@function.definition` (+ `@function.name`, `@function.parameters`, `@function.body`) | Defines a metric scope |
| `if_statement` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `repeat_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `binary_operator` `"&&"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary_operator` `"||"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `call` | `@call` | +1 ABC C-count |
| `binary_operator` `<-` / `<<-` / `=` / `->` / `->>` / `:=` | `@assign` | +1 ABC A-count |

## Assertion patterns

| Pattern | Matches |
|---------|---------|
| `^expect_` | `expect_equal(...)`, `expect_error(...)`, … (testthat) |
| `^test_that` | `test_that(...)` blocks |
| `^verify` | `verify(...)` |
| `^assert_` | `assert_that(...)`-family and `assert_*` helpers |
| `^stopifnot` | `stopifnot(...)` |

## Known unsupported metrics

- **Type definitions and fields are not captured** — *Not supported by the
  grammar*: R has no nominal type declarations, so no `@type.*` queries
  exist for R; zero captures are deliberate per the archived data-model
  design (D2).
- **`switch` is not captured** — *Not implemented yet*: R's `switch(...)` is
  an ordinary call and contributes only an ABC C-count, no branches. No
  ticket filed yet.
- **`tryCatch`/`withCallingHandlers` are not captured as branches** — *Not
  implemented yet*: they are ordinary calls; error handlers add no cyclomatic
  or cognitive complexity. No ticket filed yet.
- **Ternary-style `ifelse` is not a branch** — *Not implemented yet*:
  `ifelse()` is a call; only the `if_statement` control form counts. No
  ticket filed yet.
- **S4/R5/R6 classes are not tracked** — *Not supported by the grammar*:
  `setClass`, `setRefClass`, and R6 `R6Class` definitions are calls and
  produce no type scopes.
- **Anonymous functions** assigned with no name (e.g. passed as arguments)
  are not captured as function scopes — *Not supported by the grammar*: an
  anonymous `function_definition` carries no name node to anchor
  `@function.name`, so their branches fall through uncaptured.
- **`min_assertions`** — *Not implemented yet*: `metrics.scm` has no
  `@assert.*` capture, so unit assertion totals are always 0 (the
  `[assertions]` patterns serve rule scopes only). No ticket filed yet.
- **Nesting depth** — *Not supported by the grammar*: tree-sitter-r 1.3.0
  wraps no bodies (branch bodies are bare statements) and the engine nests
  only `block`-kind nodes, so `nesting_max` stays 0.
- **Unwrap density** — *Not implemented yet*: the engine's risky
  error-handling detector covers Rust, Python, JavaScript/TypeScript, Java,
  and Go only; R has no matcher. No ticket filed yet.

## Quirks and limitations

- **Named functions require `name <- function`** — the name comes from the
  assignment LHS, so `assign("f", function(...))` produces no scope.
- **`if` as an expression** — R's `if` returns a value and is often used
  inline; every `if_statement` node still counts one branch, including
  value-position uses the grammar parses as `if_statement`.
- **Logical operators are double-wide only** — `&&`/`||` count as
  branches; the vectorised `&`/`|` do not.

## Smell weights

Calls to `eval` (5.0), `attach` (4.0), `assign` and `rm` (3.0), `detach`
(2.0), and `options` (1.5) carry elevated weights.

## Example output

```
R/utils.r  cyclomatic=6  cognitive=8  function_lines=28  params=2
R/model.r  cyclomatic=11  cognitive=14  function_lines=47  params=3
  ↳ fit_model()  cyclomatic=11  [threshold: 10]
```
