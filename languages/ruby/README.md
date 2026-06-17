# Ruby

## Overview

Analyses `.rb` files using a tree-sitter grammar. Tracks both instance methods
and singleton methods (class-level `self.` methods). Ruby's `unless`, `until`,
and `rescue` each count as branches alongside the more common `if` and `while`.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `method` | `@function.definition` | Defines a metric scope |
| `singleton_method` | `@function.definition` | Defines a metric scope |
| `if` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `unless` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `elsif` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `until` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `rescue` | `@branch.rescue` | +1 cyclomatic, +1 cognitive |
| `when` (in `case`) | `@branch.case` | +1 cyclomatic, +1 cognitive |
| `binary` `"&&"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary` `"||"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `binary` `"and"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary` `"or"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `call` | `@call` | +1 ABC C-count |
| `assignment` | `@assign` | +1 ABC A-count |
| `operator_assignment` | `@assign` | +1 ABC A-count |

## Assertion patterns

| Pattern | Matches |
|---------|---------|
| `^assert` | Minitest `assert(...)`, `assert_equal(...)`, etc. |
| `^expect` | RSpec `expect(...)` |
| `^should` | RSpec legacy `should` syntax |
| `^raise_error` | RSpec `raise_error` matcher |

## Quirks and limitations

- **Modifier forms** (`do_something if condition`, `value unless flag`) are
  parsed as `if`/`unless` nodes and count as branches.
- **Both `&&`/`||` and `and`/`or`** are tracked — each contributes one
  cyclomatic point. Code mixing both keyword and operator forms may count
  more than expected.
- **Blocks** (`do...end`, `{...}`) are not tracked as function scopes.
- **`proc` and `lambda`** expressions are not tracked as function scopes.
- **`case/in`** pattern matching (Ruby 3.0+) may not be fully parsed by the
  current grammar revision.

## Example output

```
lib/parser.rb  cyclomatic=8  cognitive=11  function_lines=39  params=2
spec/parser_spec.rb  min_assertions=0  [threshold: 1]  (test role)
```
