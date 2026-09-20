# update-min-assertions-scope — Tasks

## 1. Red tests (TDD)

- [x] 1.1 Failing test: Rust `#[test]` fn with zero assertions still flagged by `min_assertions`
- [x] 1.2 Failing test: fixture-style unit (no attribute, name `simple`) in a test-role path NOT flagged by `min_assertions`
- [x] 1.3 Failing test: helper unit (no attribute, name `tempdir`) in a test-role path NOT flagged by `min_assertions`
- [x] 1.4 Failing test: Python `test_foo` with zero assertions still flagged
- [x] 1.5 Regression check: fixture/helper units still flagged by `function_lines_max`/`cyclomatic_max` when over threshold

## 2. Implement (green)

- [x] 2.1 Capture test attribute in `languages/rust/metrics.scm` (`#[test]` → `@testattr.rust`)
- [x] 2.2 Add test-identity evaluation to `min_assertions` rule path in `engine.rs` (attribute OR name pattern)
- [x] 2.3 Make tests from step 1 pass

## 3. Verify

- [x] 3.1 Full test suite green (cargo test, all targets)
- [x] 3.2 Meter: dogfood self-scan `min_assertions` events ≤ 5 (from 134); config.toml and language TOMLs byte-identical to base commit (anti-goal)
- [x] 3.3 `pretender check . --staged` clean (advisory exit 0)

## 4. Ship

- [x] 4.1 Feature branch + PR referencing pretender-68j
- [x] 4.2 CI green; merge; close pretender-68j
