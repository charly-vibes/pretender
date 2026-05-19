---
tags: [pipeline-run:tdd-ro5u-2026-05-19-run, pipeline-step:green]
---

GREEN: JS/TS plugins implemented. Key decisions: (1) JS grammar has no 'elif' concept — removed @branch.elif entirely; else-if chains count as nested @branch.if, consistent with cyclomatic semantics. (2) Both plugins use identical .scm query structure; TS extends with type_annotation and type_arguments captures (no metric impact). (3) tree-sitter-javascript=0.23.1 and tree-sitter-typescript=0.23.2 resolve correctly against tree-sitter=0.23 API. (4) smell_weights for eval/Function defined in plugin.toml but not wired to engine (engine hardcodes 1.0 for all calls — deferred).
