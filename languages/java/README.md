# Java

## Overview

Analyses `.java` files using a tree-sitter grammar. Tracks both method
declarations and constructors as function scopes. Enhanced `for` loops and
`switch` statement groups each count as separate branches.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `method_declaration` | `@function.definition` | Defines a metric scope |
| `constructor_declaration` | `@function.definition` | Defines a metric scope |
| `if_statement` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `enhanced_for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `do_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `catch_clause` | `@branch.catch` | +1 cyclomatic, +1 cognitive |
| `ternary_expression` | `@branch.ternary` | +1 cyclomatic, +1 cognitive |
| `switch_block_statement_group` / `switch_label` | `@branch.case` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"&&"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"||"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `method_invocation` | `@call` | +1 ABC C-count |
| `object_creation_expression` | `@call` | +1 ABC C-count |
| `assignment_expression` | `@assign` | +1 ABC A-count |
| `variable_declarator` (with value) | `@assign` | +1 ABC A-count |

## Assertion patterns

| Pattern | Matches |
|---------|---------|
| `^assert` | JUnit `assertNotNull(...)`, etc. |
| `^Assert` | `Assert.assertEquals(...)` (static import) |
| `^verify` | Mockito `verify(...)` |
| `^Verify` | `Verify.that(...)` |
| `^expect` | `expectedException.expect(...)` |
| `^fail` | `fail("message")` |

## Quirks and limitations

- **`instanceof` checks** inside `if` conditions are counted as regular
  `if_statement` branches — no special handling for pattern-matching
  `instanceof` (Java 16+).
- **Lambda expressions** are not tracked as function scopes; their branches
  count toward the enclosing method's complexity.
- **`switch` expressions** (Java 14+) may not be fully parsed by the current
  grammar revision — verify with a test file if you rely on switch-expression
  branch counts.
- **`object_creation_expression`** (`new Foo(...)`) counts as a call for
  ABC C-count, which may inflate scores in builder-heavy code.
- **Mutation testing** (`pretender mutation`) does not yet run PIT automatically.
  See [Mutation testing](../../docs/mutation.md) for the manual workflow.

## Example output

```
src/main/java/Service.java  cyclomatic=9  cognitive=13  function_lines=47  params=3
src/main/java/Parser.java  cyclomatic=15  cognitive=21  function_lines=68  params=4
  ↳ parse()  cyclomatic=15  [threshold: 10]
```
