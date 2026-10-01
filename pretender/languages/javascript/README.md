# JavaScript

Reference for which tree-sitter nodes pretender tracks for JavaScript files,
how each contributes to `pretender check` metrics, and known measurement gaps.
Consult this page when tuning thresholds or diagnosing unexpected scores.

## Overview

Analyses `.js`, `.jsx`, `.mjs`, and `.cjs` files using a tree-sitter grammar.
Tracks named functions, arrow functions assigned to variables, and class
methods. `switch_case` nodes each count as a branch.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `function_declaration` | `@function.definition` | Defines a metric scope |
| `variable_declarator` with `arrow_function` | `@function.definition` | Defines a metric scope |
| `method_definition` | `@function.definition` | Defines a metric scope |
| `if_statement` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `switch_case` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `for_in_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `do_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `catch_clause` | `@branch.catch` | +1 cyclomatic, +1 cognitive |
| `ternary_expression` | `@branch.ternary` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"&&"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"||"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `call_expression` | `@call` | +1 ABC C-count |
| `assignment_expression` | `@assign` | +1 ABC A-count |
| `augmented_assignment_expression` | `@assign` | +1 ABC A-count |
| `variable_declarator` (with value) | `@assign` | +1 ABC A-count |

## Assertion patterns

| Pattern | Matches |
|---------|---------|
| `^assert$` | Node.js `assert(...)` |
| `^assert[A-Z]` | `assertEqual(...)`, etc. |
| `^expect$` | Jest / Vitest `expect(...)` |
| `^it$` | `it('...', ...)` test blocks |
| `^test$` | `test('...', ...)` blocks |
| `^describe$` | `describe('...', ...)` blocks |

## Quirks and limitations

- **Anonymous arrow functions** not assigned to a named variable are not
  tracked as function scopes; their branches count toward the enclosing scope.
- **`??` (nullish coalescing)** is not currently captured as a branch and
  does not contribute to cyclomatic complexity.
- **Optional chaining** (`?.`) is not tracked.
- **`switch_case` nodes** include both `case X:` and `default:` labels —
  `default` is counted as one branch.
- **Shebangs** (`#!/usr/bin/env node`) are stripped by the grammar and do not
  affect parsing.

## Example output

```
src/router.js  cyclomatic=8  cognitive=12  function_lines=44  params=3
src/utils.js  cyclomatic=3  cognitive=4  function_lines=18  params=1
```
