## 1. Configuration

- [ ] 1.1 Add `inheritance_depth_max: u32` to `[thresholds]` in `config.rs`; default `0`. Validate: non-negative u32.
- [ ] 1.2 Unit tests for config parse + defaults + validation.

## 2. Inheritance depth detection

- [ ] 2.1 Extend `CodeUnit` in the universal model with an optional `parent_class: Option<String>` field (name of the direct superclass).
- [ ] 2.2 For each language adapter, add `@class.superclass` capture to class-definition queries:
  - Java: `extends SomeClass`
  - Python: `class A(B)`
  - TypeScript/JavaScript: `extends SomeClass`
  - C++: `: public SomeClass`
  - C#: `: SomeClass`
  - Ruby: `< SomeClass`
  - Rust: **excluded from depth counting**. Rust `trait A: B` with stdlib traits (Display, Debug, Clone) SHALL NOT be counted. Custom trait inheritance SHALL count as depth max 1 (not transitive).
- [ ] 2.3 Compute inheritance depth: walk the `parent_class` chain within the same file. If the parent class is not defined in the same file, depth = 1 with a note in diagnostic output. Cross-file chains are NOT resolved. Flag when depth > `inheritance_depth_max`.
- [ ] 2.4 Exclude files with role `generated` or `vendor` from inheritance-depth analysis.
- [ ] 2.5 Unit tests per language: class with depth 3 and threshold 2 flagged; depth 2 and threshold 2 not flagged; no parent class depth 0; Rust trait bound with stdlib Display NOT flagged; Rust custom trait inheritance counted as max depth 1.

## 3. Evaluation and reporting

- [ ] 3.1 Run inheritance depth computation during `check` for every file.
- [ ] 3.2 Add `inheritance_depth` violations to `UnitReport.violations`. Extend `decide_exit_code` for gate mode.
- [ ] 3.3 Human, JSON, SARIF output formats.

## 4. Validation

- [ ] 4.1 Run `openspec validate add-inheritance-depth-check --strict`.
- [ ] 4.2 Run `just ci` green.