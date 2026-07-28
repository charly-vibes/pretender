## 1. Configuration

- [ ] 1.1 Add `duration_max_ms: u32` to new `[thresholds.unit-test]` and
  `[thresholds.integration-test]` tables in `config.rs`; default `0`.
- [ ] 1.2 Add `UnitTest` and `IntegrationTest` variants to the `Role` enum in
  `roles.rs`; extend `EffectiveThresholds::for_role` to inherit `test`
  thresholds and overlay `duration_max_ms`.
- [ ] 1.3 Add `test_cmd: Option<String>`, `test_report_path: Option<String>`,
  `test_timeout_s: u32` (default `600`), and
  `test_time_unit: TimeUnit` (default `Seconds`) to the `Execute` struct in
  `config.rs`.
- [ ] 1.4 Add `[roles.unit-test]` and `[roles.integration-test]` matchers to
  the `Roles` struct with default path globs (`tests/unit/**`,
  `tests/integration/**`); add `classname_root: String` (default `tests`) to
  the `RoleMatcher` used by `[roles.test]`.
- [ ] 1.5 Validate `duration_max_ms` and `test_timeout_s` are non-negative
  u32; validate `test_time_unit` is `"seconds"` or `"milliseconds"`.
- [ ] 1.6 Unit tests for config parse + defaults + validation (extend
  `config.rs` tests).

## 2. Sub-role detection

- [ ] 2.1 Extend `RoleDetector` to register `unit-test` / `integration-test`
  globs and resolve them via the existing specificity ordering.
- [ ] 2.2 Extend `Role::parse` to accept `unit-test`/`unit_test`/`unit` and
  `integration-test`/`integration_test`/`integration`.
- [ ] 2.3 Add test-rooted path heuristics: `tests/unit/**`, `test/unit/**` →
  `unit-test`; `tests/integration/**`, `test/integration/**` →
  `integration-test`; base `test` otherwise. (Do NOT match the broad
  `**/unit/**`.)
- [ ] 2.4 Unit tests for pragma (incl. underscore and bare forms), glob, and
  heuristic resolution of sub-roles; assert `src/unit/helper.rs` is NOT
  misclassified (extend `roles.rs` tests).

## 3. JUnit XML report parsing

- [ ] 3.1 Pick a JUnit XML parser dependency (`quick-junit` vs minimal
  `quick-xml`); add to `Cargo.toml`.
- [ ] 3.2 Create `pretender/src/test_report.rs` exposing
  `parse_junit(path, time_unit) -> Vec<TestTiming>` where `TestTiming`
  carries `name`, `classname`, `file: Option<PathBuf>`, `duration_ms: u32`,
  and a `status` (passed/skipped/failed/errored).
- [ ] 3.3 Convert `time` per `test_time_unit`: seconds (default, XSD) or
  milliseconds; round-half-up to u32 milliseconds.
- [ ] 3.4 Map testcase → candidate source file: `file` attribute first; else
  replace `.` in `classname` with `/`, append the detected language
  extension, and search under `[roles.test] classname_root` (default
  `tests`), first existing file wins; unresolved → base `test` role with a
  note.
- [ ] 3.5 Skip malformed entries with a warning diagnostic; never abort the
  whole check on a single bad entry.
- [ ] 3.6 Exclude testcases with a `<skipped>` child; still evaluate
  `<failure>`/`<error>` testcases; evaluate each `<testcase>` entry
  independently (no rerun aggregation).
- [ ] 3.7 Unit tests for parsing + conversion + mapping + malformed-entry +
  skipped/failed + classname-unresolved handling using fixture JUnit XML.

## 4. Duration check evaluation

- [ ] 4.1 Create a duration-evaluation function that, given `Vec<TestTiming>`
  and a `RoleDetector` + `Config`, emits a duration finding per over-threshold
  testcase (comparison: `rounded_ms > duration_max_ms`, strict).
- [ ] 4.2 Resolve each testcase's role; apply `duration_max_ms` from the
  sub-role (or skip if base `test` / threshold `0`).
- [ ] 4.3 Add a `TestDurationFinding` struct and a top-level
  `test_findings: Vec<TestDurationFinding>` field to `CheckReport`
  (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`); render via a
  `kind = duration` discriminator so human/json/sarif render them uniformly.
- [ ] 4.4 Extend `decide_exit_code` to treat a non-empty `test_findings` as a
  violation in `gate` mode (mirror the existing `file_violations` /
  `unit.violations` check).
- [ ] 4.5 Unit tests for the evaluator covering: over-threshold unit,
  over-threshold integration, under-threshold, exactly-at-threshold (no
  finding), base-test no-threshold, zero-threshold-disabled, skipped
  exclusion, failed-still-evaluated, independent rerun entries.

## 5. CLI wiring

- [ ] 5.1 Add `--test-report <path>` and `--execute` flags to `CheckArgs` in
  `main.rs`.
- [ ] 5.2 When `--test-report` is set, parse the report and run the duration
  evaluator alongside structural analysis.
- [ ] 5.3 When `--execute` is set and `--test-report` is NOT set, run
  `[execute] test_cmd` from the repo root with inherited env, bounded by
  `test_timeout_s`; on timeout emit a `test-cmd-timeout` error finding and
  fail gate mode; on non-zero `test_cmd` exit emit an error finding and
  proceed on whatever `test_report_path` contains. Error clearly when
  `test_cmd` is unset.
- [ ] 5.4 Persist duration findings in the report cache so `pretender report`
  surfaces them.
- [ ] 5.5 Integration tests: CLI `--test-report` against a fixture JUnit XML
  produces a duration finding in human/json/sarif; `--execute` runs a stub
  command writing a fixture report; `--test-report` + `--execute` together
  does NOT run `test_cmd`.

## 6. Output formats

- [ ] 6.1 Human format: render duration findings with test name, file, role,
  observed ms, and threshold ms.
- [ ] 6.2 JSON format: include duration findings with the discriminator field.
- [ ] 6.3 SARIF format: emit duration findings as results with a dedicated
  rule id (`pretender/test-duration`).
- [ ] 6.4 Integration tests asserting each format contains the duration
  finding for the fixture from 5.5.

## 7. Docs and validation

- [ ] 7.1 Document `[thresholds.unit-test]`, `[thresholds.integration-test]`,
  `duration_max_ms`, `[execute] test_cmd` / `test_report_path` /
  `test_timeout_s` / `test_time_unit`, `[roles.test] classname_root`, and the
  `--test-report` / `--execute` flags in user docs.
- [ ] 7.2 Document JUnit XML emission incantations for common runners
  (pytest, cargo via cargo2junit, maven, gradle, jest) and the `time`-unit
  assumption per runner.
- [ ] 7.3 Run `openspec validate add-test-duration-check --strict` and
  resolve every issue.
- [ ] 7.4 Run `just ci` (fmt + type-check + clippy + test) green.
