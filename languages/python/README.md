# Python

Reference for which tree-sitter nodes pretender tracks for Python files, how
each contributes to `pretender check` metrics, and known measurement gaps.
Consult this page when tuning thresholds or diagnosing unexpected scores.

## Overview

Analyses `.py` files using a tree-sitter grammar. Tracks all function forms
(top-level, methods, nested, decorated) and Python-specific branch constructs
including `elif`, `except`, and ternary expressions.

## Tracked nodes

| Tree-sitter node | Capture | Metric impact |
|-----------------|---------|---------------|
| `function_definition` | `@function.definition` | Defines a metric scope |
| `if_statement` | `@branch.if` | +1 cyclomatic, +1 cognitive |
| `elif_clause` | `@branch.elif` | +1 cyclomatic, +1 cognitive |
| `for_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `while_statement` | `@branch.loop` | +1 cyclomatic, +1 cognitive |
| `except_clause` | `@branch.catch` | +1 cyclomatic, +1 cognitive |
| `except_group_clause` | `@branch.catch` | +1 cyclomatic, +1 cognitive |
| `conditional_expression` | `@branch.ternary` | +1 cyclomatic, +1 cognitive |
| `boolean_operator` `"and"` | `@branch.logical.and` | +1 cyclomatic, +1 cognitive |
| `boolean_operator` `"or"` | `@branch.logical.or` | +1 cyclomatic, +1 cognitive |
| `call` | `@call` | +1 ABC C-count |
| `assignment` | `@assign` | +1 ABC A-count |
| `augmented_assignment` | `@assign` | +1 ABC A-count |

## Assertion patterns

The following call patterns are recognised as assertions for the `min_assertions`
metric (test role):

| Pattern | Matches |
|---------|---------|
| `^assert$` | `assert(expr)` call form; the keyword statement `assert expr` is captured separately by the `assert_statement` tree-sitter node |
| `^assert[A-Z]` | `assertEqual`, `assertRaises`, `assertIn`, etc. |
| `^self\.assert` | `self.assertEqual(...)`, etc. |
| `^pytest\.raises$` | `pytest.raises(...)` |
| `^pytest\.warns$` | `pytest.warns(...)` |

## Quirks and limitations

- **Decorators** do not contribute to complexity — only the function body does.
- **Comprehensions** (`[x for x in y if cond]`) contain implicit branches but
  are not currently tracked; complexity may be understated for comprehension-heavy
  code.
- **`match`/`case`** (Python 3.10+) structural pattern matching arms are not
  yet captured; each arm contributes no cyclomatic count.
- **`except*`** (`except_group_clause`) is captured and contributes the same as
  a plain `except`.

## Example output

```
src/parser.py  cyclomatic=12  cognitive=18  function_lines=55  params=3
  ↳ parse_token()  cyclomatic=12  [threshold: 10]

src/parser.py:42  min_assertions=0  [threshold: 1]  (test role)
```
