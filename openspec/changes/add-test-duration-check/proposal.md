# Change: Add test-duration check

## Why

Pretender currently only performs *static* structural analysis. There is no
guard against slow tests — a unit test that takes seconds or an integration
test that takes minutes erodes the fast-feedback loop that pretender is meant
to protect. Teams need a language-agnostic gate that fails when a test
exceeds a role-appropriate duration budget (e.g. unit tests in milliseconds,
integration tests sub-second), reported through the same `check` surface and
output formats as the existing structural metrics.

## What Changes

- Add a **dynamic** check that consumes per-test timing data from a JUnit
  XML report and flags tests whose elapsed time exceeds a configurable
  per-role duration threshold.
- Introduce two test **sub-roles** — `unit-test` and `integration-test` —
  that refine the existing `test` role, detected with the same resolution
  order as today (pragma → configured glob → heuristic → fallback).
- Add `[thresholds.unit-test]` and `[thresholds.integration-test]` tables
  with a `duration_max_ms` field (u32, milliseconds).
- Add an **execute mode** that runs the configured `[execute] test_cmd`
  (expected to produce a JUnit XML report at `[execute] test_report_path`)
  and then analyses it. This **establishes** the `[execute]` command-running
  contract — the existing `coverage_cmd` and `mutation_cmd` fields are
  config-only and never executed today; test-duration is the inaugural
  consumer, so the run contract (cwd, env, timeout) is specified here.
- Add `--test-report <path>` and `--execute` flags to `pretender check`.
- Emit duration violations as findings in `human`, `json`, and `sarif`
  output, and count them toward `gate`-mode exit status via an extended
  `decide_exit_code`.
- Duration findings attach to a new top-level `test_findings` vector on
  `CheckReport` (per-test-case findings do not fit the existing
  file/unit tree).

## Impact

- Affected specs: `cli-and-config` (MODIFIED Configuration Schema, Role
  Detection, Check Command; ADDED Test Duration Check, Test Sub-Role
  Detection, Test Report Parsing).
- Affected code: `pretender/src/config.rs` (new threshold tables + sub-role
  matchers + `Execute` fields incl. `test_timeout_s` and `test_time_unit`),
  `pretender/src/roles.rs` (new `UnitTest` / `IntegrationTest` variants +
  detection), `pretender/src/main.rs` (`check` flags + duration evaluation
  path + `decide_exit_code` extension + `CheckReport.test_findings`), new
  `pretender/src/test_report.rs` (JUnit XML parser), `pretender/src/metrics.rs`
  or a new duration module (threshold comparison), SARIF/JSON report shapes.
- New dependency: a JUnit XML parser. Evaluate `quick-junit` or a minimal
  `quick-xml`-based parser; keep the surface narrow (testsuite, testcase,
  `time` attribute, `classname`/`name`/`file` mapping).
- No change to the universal code model — durations come from a report, not
  the AST, so `universal-code-model` is untouched.
- Non-breaking: the new check is opt-in. With no `--test-report`, no
  `[execute] test_cmd`, and no duration thresholds configured, `check`
  behaves exactly as today.
