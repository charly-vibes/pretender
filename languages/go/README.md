# Go

## Overview

Analyses `.go` files using a tree-sitter grammar. Tracks both top-level
functions and method declarations. Go's `select`/`case` and type-switch
cases each count as a branch, matching Go's concurrency and type-dispatch
idioms.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `function_declaration` | `@function.definition` | Defines a metric scope |
| `method_declaration` | `@function.definition` | Defines a metric scope |
| `if_statement` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `expression_case` (in switch) | `@branch.case` | +1 cyclomatic, +1 cognitive |
| `type_case` (in type switch) | `@branch.case` | +1 cyclomatic, +1 cognitive |
| `communication_case` (in select) | `@branch.case` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"&&"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"||"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `call_expression` | `@call` | +1 ABC C-count |
| `assignment_statement` | `@assign` | +1 ABC A-count |
| `short_var_declaration` | `@assign` | +1 ABC A-count |
| `var_declaration` | `@assign` | +1 ABC A-count |

## Assertion patterns

| Pattern | Matches |
|---------|---------|
| `^assert` | `assert*` (testify-style) |
| `^require` | `require.*` (testify require package) |
| `^t\.Error` | `t.Error(...)`, `t.Errorf(...)` |
| `^t\.Fatal` | `t.Fatal(...)`, `t.Fatalf(...)` |
| `^t\.Fail` | `t.Fail()`, `t.FailNow()` |

## Quirks and limitations

- **Go has no `else if` node** — `else if` chains are parsed as nested
  `if_statement` nodes, so each branch is counted independently (which is
  the intended behaviour).
- **No ternary operator** — Go does not have `?:`, so no ternary branches
  are ever counted.
- **`default` case** in a switch is not captured as a branch — only
  explicit `case` labels contribute.
- **`select` blocks** used for concurrency can produce high branch counts
  in functions that multiplex many channels.

## Example output

```
pkg/handler.go  cyclomatic=7  cognitive=9  function_lines=42  params=2
internal/router.go  cyclomatic=11  cognitive=15  function_lines=48  params=3
  ↳ Route()  cyclomatic=11  [threshold: 10]
```
