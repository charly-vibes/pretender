# C

Reference for which tree-sitter nodes pretender tracks for C files, how
each contributes to `pretender check` metrics, and known measurement gaps.
Consult this page when tuning thresholds or diagnosing unexpected scores.

## Overview

Analyses `.c` and `.h` files using a tree-sitter grammar. Tracks standard
C function definitions (not declarations). Header files are included so
complexity in inline or `static` functions defined in headers is measured.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `function_definition` | `@function.definition` | Defines a metric scope |
| `if_statement` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `do_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `case_statement` | `@branch.case` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"&&"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"||"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `call_expression` | `@call` | +1 ABC C-count |
| `assignment_expression` | `@assign` | +1 ABC A-count |
| `init_declarator` (with value) | `@assign` | +1 ABC A-count |

## Assertion patterns

| Pattern | Matches |
|---------|---------|
| `^assert` | `assert(...)` from `<assert.h>` |
| `^ASSERT` | Custom assertion macros |
| `^TEST_ASSERT` | Unity / CUnit style macros |

## Quirks and limitations

- **Function pointers** and **macro-defined functions** are not tracked as
  function scopes.
- **`goto`** is not captured as a branch — complexity in `goto`-heavy code is
  understated. The smell-weight system flags `goto` separately.
- **`default:` in switch** is captured as a `case_statement` and counted as
  one branch.
- **Header files** (`.h`) are analysed like `.c` files. Forward declarations
  (without a body) are not captured; only full definitions are tracked.
- **Preprocessor conditionals** (`#ifdef`, `#if`) are not parsed by the
  tree-sitter grammar and do not contribute branch counts.

## Example output

```
src/parser.c  cyclomatic=11  cognitive=14  function_lines=52  params=3
  ↳ parse_token()  cyclomatic=11  [threshold: 10]
include/utils.h  cyclomatic=2  cognitive=2  function_lines=8  params=1
```
