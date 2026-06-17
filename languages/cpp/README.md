# C++

## Overview

Analyses `.cpp`, `.cc`, `.cxx`, `.hpp`, and `.hxx` files using a tree-sitter
grammar. Tracks both free functions and out-of-class method definitions
(qualified identifiers). Range-based `for` loops and `catch` clauses each
count as branches.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `function_definition` (simple identifier) | `@function.definition` | Defines a metric scope |
| `function_definition` (qualified identifier) | `@function.definition` | Defines a metric scope |
| `if_statement` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `for_range_loop` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `do_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `catch_clause` | `@branch.catch` | +1 cyclomatic, +1 cognitive |
| `case_statement` | `@branch.case` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"&&"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `binary_expression` `"||"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `call_expression` | `@call` | +1 ABC C-count |
| `assignment_expression` | `@assign` | +1 ABC A-count |
| `init_declarator` (with value) | `@assign` | +1 ABC A-count |

## Assertion patterns

| Pattern | Matches |
|---------|---------|
| `^assert` | `assert(...)` from `<cassert>` |
| `^ASSERT` | Custom assertion macros |
| `^EXPECT` | Google Test `EXPECT_EQ(...)`, etc. |
| `^CHECK` | `CHECK(...)` (Abseil, glog) |

## Quirks and limitations

- **Template functions** produce one definition entry per instantiation in some
  grammar revisions; complexity may be overcounted in heavily templated code.
- **Lambda expressions** are not tracked as function scopes — their branches
  count toward the enclosing function's complexity.
- **Operator overloads** (`operator+`, `operator==`) are captured as function
  definitions and metriced like regular functions.
- **`ternary_expression`** (`condition ? a : b`) is not currently tracked and
  does not contribute cyclomatic complexity.
- **In-class method definitions** (defined inside a class body in a header) use
  the simple-identifier form and are tracked correctly. Out-of-class definitions
  (e.g., `Foo::bar()` in a `.cpp` file) use the qualified-identifier form and
  are also tracked.
- **Preprocessor conditionals** (`#ifdef`, `#if`) are not parsed by the
  tree-sitter grammar and do not contribute branch counts.

## Example output

```
src/engine.cpp  cyclomatic=10  cognitive=13  function_lines=49  params=3
include/parser.hpp  cyclomatic=5  cognitive=6  function_lines=24  params=2
```
