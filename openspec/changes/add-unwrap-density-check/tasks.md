## 1. Configuration

- [ ] 1.1 Add `unwrap_max: u32` to `[thresholds]` in `config.rs`; default `0`. Support role-specific overrides via `[thresholds.<role>] unwrap_max`.
- [ ] 1.2 Define recommended default thresholds in documentation: `[thresholds.app] unwrap_max = 0`, `[thresholds.test] unwrap_max = 5`.
- [ ] 1.2 Unit tests for config parse + defaults + role-specific overrides.

## 2. Unwrap/except detection

- [ ] 2.1 Create `pretender/src/error_metrics.rs` exposing `detect_unwraps(module, source, language) -> Vec<UnwrapSite>`, where `UnwrapSite` carries `name: String`, `span: Span`, and `kind: UnwrapKind` (`Unwrap`, `Expect`, `BareExcept`, `EmptyCatch`, `IgnoredError`).
- [ ] 2.2 Per-language detection patterns:
  - Rust: `.unwrap()` and `.expect(` call expressions (capture via `@call.callee` matching `unwrap`/`expect`)
  - Python: bare `except:`, `except Exception:` (capture via `@catch` with no type)
  - JavaScript/TypeScript: `catch {}` with no handler body, `catch(e) {}` with no rethrow or logging
  - Java: `catch (Exception e) {}` with no handler body
  - Go: `_ = fn()` (ignored error), `panic(` (bare panic), `recover()` without error check
- [ ] 2.3 Add `@call.unwrap`, `@catch`, `@catch.type` capture queries to each language adapter.
- [ ] 2.4 Exclude files with role `generated` or `vendor` from unwrap-density analysis.
- [ ] 2.5 Unit tests per language: fixture files with unwraps/excepts detected; safe error handling not flagged; Go `if err != nil { return err }` NOT flagged (idiomatic).

## 3. Evaluation and reporting

- [ ] 3.1 Run unwrap detection during `check` for every file. Count unwrap sites per code unit. Compare against role-specific `unwrap_max`.
- [ ] 3.2 Add `unwrap` violations to `UnitReport.violations`. Extend `decide_exit_code` for gate mode.
- [ ] 3.3 Human, JSON, SARIF output formats.

## 4. Validation

- [ ] 4.1 Run `openspec validate add-unwrap-density-check --strict`.
- [ ] 4.2 Run `just ci` green.