---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-b5o-universal-code-model-types, pipeline-step:plan]
---

pretender-b5o universal code model types plan: Desired end state is model.rs defines the OpenSpec baseline data model (Span, Parameter, CallSite, Import, Language, Module, CodeUnit, UnitKind, Block, Node, Branch, BranchKind) with serde Serialize/Deserialize for JSON, Module includes imports, Python/parser code compiles against updated variants, and fixtures exist for Python/Rust/JS. Out of scope: full tree-sitter query engine, new language parsers, metric implementations beyond keeping current cyclomatic tests green. Phases: RED add model serialization/fixture tests; GREEN add serde derives and missing fields/variants; RO5U review for spec alignment and edge cases; VERIFY run cargo test -p pretender.
