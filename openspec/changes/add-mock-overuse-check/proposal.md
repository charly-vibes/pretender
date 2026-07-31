# Change: Add mock-overuse check

## Why

Heavy mocking in unit tests is a well-known signal of Violated Dependency Inversion — the production code lacks seams for test doubles, forcing test authors to either mock deeply or write fragile integration-like unit tests. Pretender currently has no way to detect or gate on mock overuse, leaving teams without a deterministic, CI-gateable signal for this structural smell.

## What Changes

- Add a **static** check that counts mock, stub, fake, and spy references in test files and flags files exceeding a configurable per-role threshold.
- Introduce `mock_count_max` to the `[thresholds.test]` config table (default `0` = disabled).
- Detect mock references via per-language patterns: known mock libraries (e.g., `mock`, `Mock`, `#[mock]`, `mockito`), import patterns, and inline mock/test-double constructions.
- Distinguish between mock **infrastructure** references (imports, trait annotations) and mock **usage** references (`mock(\"GET\", \"/api\")`, `expect(...)`, `Mock::new()`). Only usage references count toward the threshold.
- User-extensible mock pattern list via `[patterns.mock] extra = ["my_lib::Mock"]` in config.
- Integrate with the existing role system — only files with `test` role (and future sub-roles) are evaluated. Files with role `generated` or `vendor` are excluded.
- Mock-reference metadata attaches to `FileReport` as a new `mock_findings` field, rendered in `human`, `json`, and `sarif` output formats, and counted toward `gate`-mode exit status.
- The check is purely static (AST + import analysis) — no test execution required.

## Impact

- Affected specs: `cli-and-config` (MODIFIED Thresholds Schema, Check Report; ADDED Mock-Overuse Detection).
- Affected code: `pretender/src/config.rs` (new `mock_count_max` threshold field), `pretender/src/main.rs` (mock-overuse evaluation path in `check`, extended `decide_exit_code`, new `mock_findings` on `FileReport`), new `pretender/src/mock_detector.rs` (per-language mock-reference patterns + counting logic).
- Affects all currently supported languages equally (each language adapter may need mock library patterns).
- No change to the universal code model — mock references are counted from per-language patterns, not from the AST model.
- Non-breaking: default `mock_count_max = 0` disables the check entirely.

## Dependencies

- None (independent). Shares the `language_patterns` registry with other checks per CONVENTIONS.md.