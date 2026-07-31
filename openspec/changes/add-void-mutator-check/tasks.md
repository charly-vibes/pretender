## 1. Configuration

- [ ] 1.1 Add `void_mutators_max: u32` to `[thresholds]` in `config.rs`; default `0`. Support role-specific overrides via `[thresholds.<role>] void_mutators_max`.
- [ ] 1.2 Unit tests for config parse + defaults + role-specific overrides.

## 2. Void mutator detection

- [ ] 2.1 Extend `pretender/src/mutability_metrics.rs` (shared with mutable-state proposal) with `detect_void_mutators(module, source, language) -> Vec<VoidMutator>`, where `VoidMutator` carries `name: String`, `span: Span`, and `mutation_count: u32`.
- [ ] 2.2 Per-language void-mutator detection:
  - Rust: `fn ...(&mut self)` with no return type (or `-> ()`) and `self.` direct assignment in body
  - Python: `def ...(self,` with no `return` statement and `self.` direct assignment
  - Java: `void` method with `this.` direct assignment in body
  - TypeScript/JavaScript: method with no `return` and `this.` direct assignment
  - C++: `void` method with `this->` direct assignment
- [ ] 2.3 Add `@self.assign` capture to language adapter queries where direct assignment to `self`/`this` fields occurs. Do NOT capture method calls on child objects (`self.list.add(x)`) — only direct field assignment (`self.field = value`).
- [ ] 2.4 Exclude files with role `generated` or `vendor` from void-mutator analysis.
- [ ] 2.5 Unit tests per language: fixture with void mutators detected; pure function not detected; return-new-instance method not detected; transitive mutation via child method call NOT flagged.

## 3. Evaluation and reporting

- [ ] 3.1 Run void-mutator detection during `check` for every non-test-role file (or per role threshold).
- [ ] 3.2 Count void mutators per file. Compare against role-specific `void_mutators_max`.
- [ ] 3.3 Add `void_mutator` violations to `UnitReport.violations`. Extend `decide_exit_code` for gate mode.
- [ ] 3.4 Human, JSON, SARIF output formats.

## 4. Validation

- [ ] 4.1 Run `openspec validate add-void-mutator-check --strict`.
- [ ] 4.2 Run `just ci` green.