# TypeScript

Reference for which tree-sitter nodes pretender tracks for TypeScript files,
how each contributes to `pretender check` metrics, and known measurement gaps.
Consult this page when tuning thresholds or diagnosing unexpected scores.

## Overview

Analyses `.ts`, `.tsx`, `.mts`, and `.cts` files using a tree-sitter grammar.
Identical branch and call tracking to the JavaScript adapter; type annotations
and generic type arguments are captured but carry no metric weight.

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
| `type_annotation` | `@type.annotation` | None |
| `type_arguments` | `@type.generic` | None |

## Assertion patterns

Same as JavaScript:

| Pattern | Matches |
|---------|---------|
| `^assert$` | `assert(...)` |
| `^assert[A-Z]` | `assertEqual(...)`, etc. |
| `^expect$` | Jest / Vitest `expect(...)` |
| `^it$` | `it('...', ...)` test blocks |
| `^test$` | `test('...', ...)` blocks |
| `^describe$` | `describe('...', ...)` blocks |

## Quirks and limitations

- **Type-only constructs** (`interface`, `type` aliases, `enum` declarations)
  are not tracked as function scopes.
- **Type guards** (`if (x instanceof Y)`) are counted as regular `if_statement`
  branches — no special handling.
- **`??` (nullish coalescing)** and **`?.` (optional chaining)** are not
  tracked as branches.
- **Decorators** on classes and methods do not contribute to complexity.
- TypeScript-specific syntax uses the same tree-sitter grammar revision as
  JavaScript (`.tsx` files are parsed as TSX).

## Example output

```
src/api.ts  cyclomatic=6  cognitive=8  function_lines=31  params=2
src/components/Form.tsx  cyclomatic=4  cognitive=5  function_lines=22  params=1
```
