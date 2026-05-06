# Universal Code Model

**Version:** 0.1.0  
**Status:** Baseline

The single abstraction every metric operates on. Language adapters map tree-sitter CST nodes to these types via `.scm` query captures.

## Core Types

### Module

File-level container. One `Module` per source file.

```rust
struct Module {
    path: PathBuf,
    language: Language,
    span: Span,           // (1, last_line)
    lines_total: u32,
    lines_code: u32,      // excludes blanks and comments
    lines_comment: u32,
    units: Vec<CodeUnit>,
    imports: Vec<Import>, // populated only when LSP is enabled
}
```

### CodeUnit

A callable: function, method, lambda, constructor, or initializer.

```rust
struct CodeUnit {
    name: String,
    kind: UnitKind,       // Function | Method | Lambda | Ctor | Initializer
    span: Span,
    parameters: Vec<Parameter>,
    body: Block,
    is_exported: bool,
}
```

### Block

A nested scope with its children. `nesting` is depth from the enclosing `CodeUnit` body root (0-based).

```rust
struct Block {
    span: Span,
    nesting: u32,
    children: Vec<Node>,
}
```

### Node

```rust
enum Node {
    Statement(Span),
    Branch(Branch),       // contributes +1 to cyclomatic
    NestedBlock(Block),   // contributes to nesting depth
    Call(CallSite),       // contributes to fan-out / ABC
}
```

### Branch

```rust
struct Branch {
    kind: BranchKind,
    span: Span,
    nesting_at: u32,      // nesting depth at point of branch (for cognitive weight)
}

enum BranchKind {
    If,
    ElseIf,
    SwitchCase,
    Loop,
    Catch,
    Ternary,
    LogicalAnd,
    LogicalOr,
    NullCoalesce,
    EarlyReturn,
}
```

## Tree-Sitter Query Contract

Each supported language ships one `.scm` file. The engine reads captures and builds the universal model. Capture name conventions:

| Capture | Maps to |
|---------|---------|
| `@function.definition` | `CodeUnit` (root anchor) |
| `@function.name` | `CodeUnit.name` |
| `@function.parameters` | `CodeUnit.parameters` |
| `@function.body` | `CodeUnit.body` |
| `@branch.if` | `Branch { kind: If }` |
| `@branch.elif` | `Branch { kind: ElseIf }` |
| `@branch.loop` | `Branch { kind: Loop }` |
| `@branch.switch` | `Branch { kind: SwitchCase }` |
| `@branch.catch` | `Branch { kind: Catch }` |
| `@branch.ternary` | `Branch { kind: Ternary }` |
| `@branch.logical` | `Branch { kind: LogicalAnd \| LogicalOr }` |
| `@call` | `CallSite` |
| `@assign` | assignment node (for ABC A-count) |
| `@assert.*` | assertion pattern (for min_assertions in test role) |

## Invariants

- A `CodeUnit` body always has `nesting = 0`
- `Block.nesting` increments by 1 for each nested block within a `CodeUnit`
- Branches inside nested blocks carry the nesting depth at time of capture
- `is_exported` defaults to `true` for languages without explicit visibility; adapters set it from grammar nodes where available
- Lambdas are `CodeUnit` instances only when they have a block body; expression lambdas are `Node::Call`

## Metrics as Pure Functions

```rust
fn cyclomatic(u: &CodeUnit) -> u32 { 1 + count_branches(&u.body) }

fn cognitive(u: &CodeUnit) -> u32 {
    walk(&u.body)
        .filter_map(|n| n.as_branch())
        .map(|b| b.cognitive_weight() * (1 + b.nesting_at))
        .sum()
}

fn function_lines(u: &CodeUnit) -> u32 { u.span.lines() }
fn params(u: &CodeUnit) -> u32          { u.parameters.len() as u32 }
fn nesting_max(u: &CodeUnit) -> u32 {
    walk_blocks(&u.body).map(|b| b.nesting).max().unwrap_or(0)
}

fn abc(u: &CodeUnit) -> f64 {
    let a = count_assignments(&u.body) as f64;
    let b = count_branches_weighted(&u.body); // smell-weighted
    let c = count_calls_weighted(&u.body);    // smell-weighted
    (a*a + b*b + c*c).sqrt()
}
```
