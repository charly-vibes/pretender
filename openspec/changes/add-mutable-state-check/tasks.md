## 1. Configuration

- [ ] 1.1 Add `mut_ratio_max: f64` to `[thresholds]` in `config.rs`; default `0.0`. Validate: `0.0 <= mut_ratio_max <= 1.0`.
- [ ] 1.2 Unit tests for config parse + defaults + validation.

## 2. Mutable-binding detection

- [ ] 2.1 Create `pretender/src/mutability_metrics.rs` exposing `count_mutable_bindings(module, source) -> (u32, u32)` where first is mutable count and second is total binding count.
- [ ] 2.2 Per-language binding patterns:
  - Rust: `let mut` (capture via `@mutable`), `mut` params
  - JavaScript/TypeScript: `let`, `var` (vs `const`)
  - Java: non-final local variable declarations
  - C++: `mutable` keyword, non-const variable declarations
  - Python: **excluded** — no `const` keyword, `global` is not a mutability signal
- [ ] 2.3 Add `@mutable` capture queries to each language adapter where not already present.
- [ ] 2.4 Exclude files with role `generated` or `vendor` from mutable-state analysis.
- [ ] 2.5 Divide total binding count by mutable count to compute ratio.
- [ ] 2.6 Unit tests per language with fixture files covering various binding ratios.

## 3. Evaluation and reporting

- [ ] 3.1 Run `count_mutable_bindings` during `check` for every file. Compare ratio against `mut_ratio_max`.
- [ ] 3.2 Add `mut_ratio: f64` and `mut_total: u32` / `mut_mutable: u32` optional fields to `FileReport` (serde skip when 0).
- [ ] 3.3 Emit finding when ratio > threshold. Extend `decide_exit_code` for gate mode.
- [ ] 3.4 Human, JSON, SARIF output formats include the finding.

## 4. Validation

- [ ] 4.1 Run `openspec validate add-mutable-state-check --strict`.
- [ ] 4.2 Run `just ci` green.