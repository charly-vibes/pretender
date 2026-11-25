# Julia

Reference for which tree-sitter nodes pretender tracks for Julia files, how
each contributes to `pretender check` metrics, and known measurement gaps.
Consult this page when tuning thresholds or diagnosing unexpected scores.

## Overview

Analyses `.jl` files using a tree-sitter grammar. Tracks `function` and
`macro` definitions; macros count as function scopes. Branch coverage is
generic: each `if`/`elseif` clause, loop, `try`, ternary, and short-circuit
operator counts one branch.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `function_definition` | `@function.definition` + `@function.body` | Defines a metric scope |
| `macro_definition` | `@function.definition` + `@function.body` | Defines a metric scope (treated as a function) |
| `if_statement` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `elseif_clause` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `try_statement` | `@branch.catch` | +1 cyclomatic, +1 cognitive |
| `ternary_expression` | `@branch.ternary` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"&&"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"||"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `call_expression` | `@call` | +1 ABC C-count |
| `assignment` | `@assign` | +1 ABC A-count |
| `compound_assignment_expression` | `@assign` | +1 ABC A-count |
| `macrocall_expression` (test/assert macros) | `@assert.macrocall` | Counts toward `min_assertions` |
| `struct_definition` | `@type.definition` | Defines a type scope |
| `typed_expression` | `@type.field` | Field name/type in the code model |
| `assignment` (plain, in struct) | `@type.field` | Untyped field (type name stays unset) |

## Assertion patterns

Assertions are matched on the macro name of a `macrocall_expression`:
`@test`, `@test_throws`, `@test_broken`, `@test_logs`, `@assert` (query
capture), plus the manifest patterns `^@test`, `^@inferred`,
`^@test_deprecated`, `^@test_nowarn`, `^@test_warn`, `^@test_skip`, and
`^Test\.test_`.

## Quirks and limitations

- **Julia bodies are flat** — `function` bodies are direct children of
  `function_definition` (no wrapping block), so the whole node is captured
  as both `@function.definition` and `@function.body` and the walker
  iterates child expressions.
- **`elseif` captures as `@branch.if`** — there is one branch per
  `elseif_clause`, in addition to the parent `if_statement`.
- **Comprehensions are not tracked as branches** — a `for` inside a
  comprehension or generator does not contribute complexity; only
  statement-level `for_statement` loops do.
- **No `let`/`do`-block branches** — `let` blocks and `do ... end` closure
  arguments are not captured.
- **Struct fields with default values** are detected via the plain
  `assignment` pattern; the field's type name stays unset for those.

## Smell weights

Calls to `eval` (5.0), `Meta.parse` (4.0), `ccall` (3.0), and `include`
(2.0) carry elevated weights.

## Example output

```
src/model.jl  cyclomatic=8  cognitive=10  function_lines=35  params=3
src/analysis.jl  cyclomatic=12  cognitive=17  function_lines=52  params=4
  ↳ fit!()  cyclomatic=12  [threshold: 10]
```
