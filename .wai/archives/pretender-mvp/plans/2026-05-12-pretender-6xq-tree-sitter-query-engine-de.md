---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-6xq-tree-sitter-query-engine, pipeline-step:plan]
---

## pretender-6xq: Tree-sitter query engine

### Desired end state
A `QueryEngine` that loads a `.scm` query, runs it against a tree-sitter CST, and maps @captures to the universal model — replacing manual CST traversal. Python .scm file as proof. Existing metrics produce identical results.

### Phase 1: Query engine scaffolding
- New `engine.rs` module with `QueryEngine` struct
- Test: compile valid query against Python grammar succeeds
- Test: invalid query returns error

### Phase 2: Function extraction via captures
- Write `queries/python.scm` with @function.definition, @function.name, @function.parameters, @function.body
- Test: parse simple Python → 1 CodeUnit with correct name/span
- Test: multiple functions → correct count
- Test: class methods detected with correct UnitKind

### Phase 3: Branch extraction via captures
- Add @branch.* patterns to Python .scm
- Test: if/for/while → correct Branch nodes with correct BranchKind
- Test: elif → ElseIf branch
- Test: try/except → Catch branch
- Test: boolean operators → LogicalAnd/LogicalOr
- Test: ternary → Ternary branch

### Phase 4: Full fixture integration
- Test: parse_python_fixture — python_simple.py produces ≥2 CodeUnits, correct spans, correct branch captures
- Verify cyclomatic/cognitive metrics produce same results as current implementation

### Phase 5: Error handling + parser swap
- Test: malformed Python → diagnostic, no partial Module
- Replace PythonParser internals to delegate to QueryEngine
- All existing tests still pass

### Out of scope
- Other language .scm files (pretender-07m, 8n5, s7d)
- plugin.toml format
- @call / @assign captures (pretender-4eh ABC scoring)
