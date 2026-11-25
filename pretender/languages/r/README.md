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

## Unsupported metrics

- **Type definitions and fields are not captured** — R has no nominal type
  declarations, so no `@type.*` queries exist for R.
- **`switch` is not captured** — R's `switch(...)` is an ordinary call and
  contributes only an ABC C-count, no branches.
- **`tryCatch`/`withCallingHandlers` are not captured as branches** — they
  are ordinary calls; error handlers add no cyclomatic or cognitive
  complexity.
- **Ternary-style `ifelse` is not a branch** — `ifelse()` is a call; only
  the `if_statement` control form counts.
- **S4/R5/R6 classes are not tracked** — `setClass`, `setRefClass`, and
  R6 `R6Class` definitions produce no type scopes.
- **Anonymous functions** assigned with no name (e.g. passed as arguments)
  are not captured as function scopes; their branches fall through
  uncaptured.

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
