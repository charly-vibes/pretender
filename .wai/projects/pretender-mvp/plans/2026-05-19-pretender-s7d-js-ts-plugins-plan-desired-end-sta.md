---
tags: [pipeline-run:tdd-ro5u-2026-05-19-run, pipeline-step:plan]
---

pretender-s7d JS/TS plugins plan:

Desired end state:
- languages/javascript/plugin.toml + metrics.scm, wired in src/javascript.rs, dispatched for .js/.jsx
- languages/typescript/plugin.toml + metrics.scm, wired in src/typescript.rs, dispatched for .ts/.tsx
- tests/fixtures/ts_sample.ts with expected_complexity annotations
- Integration tests in cli_test.rs: complexity command on js_simple.js and ts_sample.ts pass
- Cargo.toml has tree-sitter-javascript = '0.23' and tree-sitter-typescript = '0.23'

Phases:
1. Add Cargo.toml deps; create plugin.toml files
2. Write failing integration tests (RED)
3. Write metrics.scm query files + parser modules (GREEN)
4. Rule-of-5 review
5. Verify + close

Out of scope:
- Wiring smell_weights into QueryEngine (hardcoded 1.0 engine-wide; eval/Function weights in plugin.toml but not applied)
- interface_declaration as CodeUnit::Initializer (engine determine_unit_kind is Python-centric; separate ticket)
- generic_type captures: included in .scm as structural hints, no metric contribution
