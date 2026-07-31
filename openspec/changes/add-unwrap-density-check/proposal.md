# Change: Add unwrap-density check

## Why

Bare `unwrap()` / `expect()` calls (Rust), naked `except:` / `catch{}` blocks (Python/JS), and untyped error silos are a reliable signal of weak error handling — failures that will crash or silently corrupt. A deterministic count per file lets teams enforce "no unwrap in app code" or "no bare except" as a CI gate, especially for `app`-role files.

## What Changes

- Add a per-file count of risky error-handling patterns:
  - Rust: `unwrap()`, `expect(...)`, `.unwrap()`, `.expect(...)`
  - Python: bare `except:`, `except Exception:`, `raise Exception` without custom type
  - JS/TS: empty `catch {}`, `catch(e) {}` without rethrow or logging
  - Java: `catch (Exception e) {}` without handling
  - Go: ignored error `_ = fn()` or bare `panic`/`recover`
- Introduce `unwrap_max: u32` under `[thresholds]` (default `0` = disabled). Role-specific override via `[thresholds.app] unwrap_max`, `[thresholds.test] unwrap_max` etc.
- Findings attach per `CodeUnit` in `UnitReport.violations` with metric name `unwraps`.
- Purely static — AST-level pattern matching across method bodies.

## Impact

- Affected specs: `cli-and-config` (ADDED unwrap threshold fields, violation metric name, role-specific threshold tables).
- Affected code: `pretender/src/config.rs` (new `unwrap_max` + role overrides), `pretender/src/main.rs` (evaluation in per-unit metric pass, violation creation), new `pretender/src/error_metrics.rs` (per-language unwrap detection).
- Language-specific — each adapter needs query patterns for unwrap/except/catch.
- Excludes files with role `generated` or `vendor`.
- Documents sensible default thresholds: `[thresholds.app] unwrap_max = 0`, `[thresholds.test] unwrap_max = 5`.
- Non-breaking: default `0` disables the check.

## Dependencies

- None (independent).