# Rust

Reference for which tree-sitter nodes pretender tracks for Rust files, how
each contributes to `pretender check` metrics, and known measurement gaps.
Consult this page when tuning thresholds or diagnosing unexpected scores.

## Overview

Analyses `.rs` files using a tree-sitter grammar. Tracks `fn` items (free
functions and methods). `match` arms each count as one branch, reflecting
Rust's exhaustive pattern matching.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `function_item` | `@function.definition` | Defines a metric scope |
| `if_expression` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for_expression` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while_expression` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `loop_expression` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `match_arm` | `@branch.match_arm` | +1 cyclomatic, +1 cognitive |
| `call_expression` | `@call` | +1 ABC C-count |
| `assignment_expression` | `@assign` | +1 ABC A-count |
| `compound_assignment_expr` | `@assign` | +1 ABC A-count |
| `let_declaration` (with value) | `@assign` | +1 ABC A-count |

## Assertion patterns

| Pattern | Matches |
|---------|---------|
| `^assert$` | `assert!(...)` |
| `^assert_eq$` | `assert_eq!(...)` |
| `^assert_ne$` | `assert_ne!(...)` |
| `^panic$` | `panic!(...)` |

## Quirks and limitations

- **`&&` and `||` operators are not tracked** — logical operators do not
  contribute to cyclomatic or cognitive complexity for Rust. This reflects
  Rust's idiom of chaining results and options rather than using boolean guards.
- **Closures** are not tracked as separate function scopes; their branches
  count toward the enclosing function's complexity.
- **`if let` and `while let`** are captured as `if_expression` and
  `while_expression` respectively, contributing one branch each.
- **Wildcard `_` arms** in `match` are counted as branches like any other arm.

## Example output

```
src/engine.rs  cyclomatic=9  cognitive=11  function_lines=38  params=2
src/config.rs  cyclomatic=14  cognitive=22  function_lines=61  params=4
  ↳ parse_config()  cyclomatic=14  [threshold: 10]
```
