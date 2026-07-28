## ADDED Requirements

### Requirement: Test Duration Check

The system SHALL provide a dynamic test-duration check that flags any
individual test case whose observed elapsed time exceeds the
`duration_max_ms` threshold of the role assigned to that test's source file.
The check SHALL consume timing data from a JUnit XML report, supplied either
via the `--test-report <path>` flag or produced by running `[execute]
test_cmd` when `--execute` is passed.

Duration findings SHALL be emitted through the existing `human`, `json`, and
`sarif` output formats and SHALL count toward a non-zero exit code in `gate`
mode, identically to structural threshold violations. Each duration finding
SHALL identify the test name, the resolved source file path (when available),
the assigned role, the observed duration in milliseconds, and the
applicable `duration_max_ms` threshold.
Duration findings SHALL attach to a new top-level `test_findings` vector on
`CheckReport` (serialized with `skip_serializing_if = Vec::is_empty`) rather
than to the per-file `FileReport` tree, because test cases frequently
reference source files outside the scanned `paths`. The `decide_exit_code`
function SHALL treat a non-empty `test_findings` as a violation in `gate`
mode, mirroring the existing `file_violations` / `unit.violations` checks.

The JUnit `time` attribute SHALL be interpreted as seconds (per the JUnit
XSD) by default, overridable via `[execute] test_time_unit = "seconds" |
"milliseconds"`. The value SHALL be converted to milliseconds using
round-half-up, and a finding SHALL be emitted iff the rounded milliseconds
strictly exceed `duration_max_ms` (a test exactly at threshold does not
fail).

When `--execute` is passed, the system SHALL run `[execute] test_cmd` from
the repository root with the parent process environment, bounded by
`[execute] test_timeout_s` (u32, default `600`). On timeout the system SHALL
emit a single error finding `test-cmd-timeout` and exit non-zero in `gate`
mode; a non-zero `test_cmd` exit SHALL be surfaced as an error finding and
the duration check SHALL proceed on whatever `test_report_path` contains.
`--test-report` and `--execute` MAY both be passed; when both are given,
`--test-report` SHALL take precedence and `--execute` SHALL NOT run.

When no `--test-report` is given, no `--execute` is given, and no test
sub-role has a non-zero `duration_max_ms` configured, the duration check
SHALL be inert and `pretender check` SHALL behave exactly as before this
change.

`duration_max_ms` SHALL be a non-negative integer in milliseconds; a value
of `0` SHALL mean "no limit" (the check is disabled for that role).

#### Scenario: JUnit report flags slow unit test
- **WHEN** `pretender check . --test-report results.xml` is run with a JUnit
  report whose `<testcase name="slow" time="0.350">` maps to a file
  classified as `unit-test` and `[thresholds.unit-test] duration_max_ms = 100`
- **THEN** the report contains a duration finding for `slow` with observed
  duration `350` ms and threshold `100` ms

#### Scenario: Test exactly at threshold does not fail
- **WHEN** a JUnit report contains `<testcase name="t" time="0.100">` and
  the assigned role has `duration_max_ms = 100`
- **THEN** no duration finding is emitted (the rounded `100` ms does not
  strictly exceed the threshold)

#### Scenario: Round-half-up conversion
- **WHEN** a JUnit report contains `<testcase name="t" time="0.1005">`
- **THEN** the parsed duration is `101` milliseconds (round-half-up)

#### Scenario: Milliseconds-unit report is honoured
- **WHEN** `[execute] test_time_unit = "milliseconds"` is configured and a
  report contains `<testcase name="t" time="350">`
- **THEN** the parsed duration is `350` milliseconds (not 350000)

#### Scenario: Skipped testcases are excluded
- **WHEN** a JUnit report contains a `<testcase>` with a `<skipped>` child
- **THEN** that testcase is excluded from duration evaluation entirely

#### Scenario: Failed testcases are still duration-evaluated
- **WHEN** a JUnit report contains `<testcase name="t" time="2.0">` with a
  `<failure>` child and the assigned role has `duration_max_ms = 500`
- **THEN** a duration finding is emitted for `t` (a slow-then-failing test
  is still slow)

#### Scenario: Each testcase entry is evaluated independently
- **WHEN** a JUnit report contains three `<testcase>` entries for the same
  logical test with `time` values `0.05`, `0.05`, and `0.8`, and the role has
  `duration_max_ms = 500`
- **THEN** exactly one duration finding is emitted, for the `0.8` entry

#### Scenario: Execute mode runs the suite then analyses
- **WHEN** `pretender check . --execute` is run with
  `[execute] test_cmd = "pytest --junitxml=results.xml"` and
  `test_report_path = "results.xml"` configured
- **THEN** the system runs `test_cmd`, reads `results.xml`, and includes
  duration findings in the same report as a `--test-report` run

#### Scenario: Execute command timeout fails the gate
- **WHEN** `pretender check . --execute --mode gate` is run and `test_cmd`
  runs longer than `[execute] test_timeout_s`
- **THEN** the system emits a `test-cmd-timeout` error finding and exits
  non-zero

#### Scenario: Test-report flag takes precedence over execute
- **WHEN** `pretender check . --test-report a.xml --execute` is run
- **THEN** the system analyses `a.xml` and does NOT run `test_cmd`

#### Scenario: Gate mode fails on duration violation
- **WHEN** `pretender check . --test-report results.xml --mode gate` reports
  at least one duration finding
- **THEN** the command exits with a non-zero code

#### Scenario: No report and no threshold leaves check unchanged
- **WHEN** `pretender check .` is run with no `--test-report`, no `--execute`,
  and no `duration_max_ms` configured on any test sub-role
- **THEN** the command produces no duration findings and behaves identically
  to a build without this change

#### Scenario: Zero threshold disables the role
- **WHEN** a test file is classified as `integration-test` and
  `[thresholds.integration-test] duration_max_ms = 0`
- **THEN** no duration finding is emitted for testcases mapped to that file
  regardless of their observed time

### Requirement: Test Sub-Role Detection

The system SHALL classify test files into sub-roles `unit-test` and
`integration-test` as refinements of the `test` role, using the same
resolution order as the base role detection:
1. explicit pragma `pretender: role=unit-test` or
   `pretender: role=integration-test` in the first 8 lines (the parser SHALL
   also accept the underscore forms `unit_test` / `integration_test` and the
   bare forms `unit` / `integration`);
2. configured `[roles.unit-test]` and `[roles.integration-test]` path globs,
   most specific match winning;
3. built-in path heuristics
   (`tests/unit/**` and `test/unit/**` → `unit-test`,
   `tests/integration/**` and `test/integration/**` → `integration-test`);
4. fallback to the base `test` role when no sub-role signal is present.

The heuristics are deliberately scoped to test-rooted paths (`tests/unit/**`,
`test/unit/**`) rather than the broad `**/unit/**`, so incidental
non-test directories like `src/unit/` are not misclassified.

A file assigned a sub-role SHALL inherit the `test` thresholds and overlay
only the sub-role-specific fields. Files that resolve to the base `test`
role (no sub-role signal) SHALL NOT receive a duration threshold.

#### Scenario: Pragma selects sub-role
- **WHEN** a file's first 8 lines contain `# pretender: role=unit-test`
- **THEN** the file is assigned role `unit-test` regardless of its path

#### Scenario: Underscore pragma form is accepted
- **WHEN** a file's first 8 lines contain `# pretender: role=integration_test`
- **THEN** the file is assigned role `integration-test`

#### Scenario: Glob selects sub-role over heuristic
- **WHEN** a file path matches both a configured `[roles.integration-test]`
  glob and the built-in `tests/unit/**` heuristic
- **THEN** the configured glob wins and the file is assigned
  `integration-test`

#### Scenario: Non-test unit directory is not misclassified
- **WHEN** a file path is `src/unit/helper.rs` and no pragma or configured
  sub-role glob applies
- **THEN** the file is NOT assigned `unit-test` (it falls through to the
  base `test` or `app` role per the existing pipeline)

#### Scenario: No sub-role signal falls back to base test
- **WHEN** a test file matches no sub-role pragma, glob, or heuristic
- **THEN** the file is assigned the base `test` role and no duration
  threshold applies

### Requirement: Test Report Parsing

The system SHALL parse JUnit XML reports conforming to the common
`<testsuites>/<testsuite>/<testcase>` structure and SHALL read each
`<testcase>` `time` attribute, interpreting it as seconds by default (per the
JUnit XSD) and overridable to milliseconds via
`[execute] test_time_unit = "milliseconds"`. The value SHALL be converted to
milliseconds using round-half-up. The parser SHALL tolerate the absence of
optional attributes and SHALL not fail the whole check when individual
entries are malformed; malformed entries SHALL be skipped with a warning
diagnostic.

Testcases containing a `<skipped>` child SHALL be excluded from duration
evaluation. Testcases containing `<failure>` or `<error>` children SHALL
still be duration-evaluated. Each `<testcase>` entry SHALL be evaluated
independently (no aggregation across rerun entries).

The parser SHALL map each testcase to a candidate source file path using, in
order: the `file` attribute when present; otherwise a concrete default
algorithm — replace `.` in `classname` with `/`, append the detected
language extension, and search under the configured
`[roles.test] classname_root` (default `tests`), taking the first existing
file. Testcases whose `classname` resolves to no existing file SHALL be
reported under the base `test` role with a note.

#### Scenario: time attribute is converted to milliseconds
- **WHEN** a JUnit report contains `<testcase name="t" time="0.250">`
- **THEN** the parsed duration is `250` milliseconds

#### Scenario: missing file attribute falls back to classname
- **WHEN** a `<testcase classname="unit.test_foo" name="test_foo" time="0.1">`
  has no `file` attribute, the detected language is Python, and the project
  has a `tests/unit/test_foo.py` file under the default `classname_root`
- **THEN** the parser maps the testcase to `tests/unit/test_foo.py` for role
  resolution

#### Scenario: classname resolves to no existing file
- **WHEN** a `<testcase classname="unit.test_foo" name="test_foo" time="0.1">`
  has no `file` attribute and no `tests/unit/test_foo.*` file exists under
  `classname_root`
- **THEN** the testcase is reported under the base `test` role with a note
  and no duration threshold applies

#### Scenario: malformed entry is skipped with warning
- **WHEN** a `<testcase>` element lacks a parseable `time` attribute
- **THEN** the parser skips that entry and emits a warning diagnostic, and
  the check continues with the remaining entries

## MODIFIED Requirements

### Requirement: Configuration Schema

The system SHALL read `pretender.toml` with tables for `[pretender]`,
`[thresholds]`, role-specific threshold tables such as `[thresholds.test]`,
`[thresholds.unit-test]`, `[thresholds.integration-test]`, `[bands]`,
`[scope]`, `[execute]`, `[plugins]`, `[output]`, and `[roles]`. The `mode`
value SHALL be one of `guidance`, `tiered`, or `gate`. The implicit default
role SHALL be `app`. Unknown config keys SHALL be ignored.

The `[thresholds.unit-test]` and `[thresholds.integration-test]` tables SHALL
each accept a `duration_max_ms` non-negative integer field (milliseconds);
`0` means no limit. These sub-role tables SHALL inherit all `test` threshold
fields and overlay only `duration_max_ms`.

The `[execute]` table SHALL additionally accept `test_cmd` (a shell command
expected to produce a JUnit XML report), `test_report_path` (the path to
that report), `test_timeout_s` (u32, default `600`), and `test_time_unit`
(`"seconds"` default, or `"milliseconds"`). When `--execute` is passed to
`check`, the system SHALL run `test_cmd` from the repository root with the
parent process environment, bounded by `test_timeout_s`, and then read
`test_report_path`.

The `[roles.test]` matcher SHALL additionally accept a `classname_root`
field (string, default `"tests"`) used by the JUnit parser's
`classname`-to-path resolution.

#### Scenario: Role-specific thresholds override app defaults
- **WHEN** a file is assigned role `test`
- **THEN** values under `[thresholds.test]` override app threshold defaults
  for that file

#### Scenario: Sub-role duration threshold is configured
- **WHEN** `pretender.toml` contains `[thresholds.unit-test] duration_max_ms = 100`
- **THEN** the configuration parses and the `unit-test` role's duration
  limit is `100` milliseconds

#### Scenario: Execute test command is configured
- **WHEN** `pretender.toml` contains `[execute] test_cmd = "pytest --junitxml=results.xml"` and `test_report_path = "results.xml"`
- **THEN** the configuration parses and `--execute` runs that command before
  analysing `results.xml`

#### Scenario: Unknown key is ignored
- **WHEN** `pretender.toml` contains an unknown key
- **THEN** parsing succeeds and the unknown key has no effect

### Requirement: Role Detection

The system SHALL assign each file a role from `app`, `library`, `test`,
`unit-test`, `integration-test`, `script`, `generated`, or `vendor` using the
current MVP resolution order:
1. explicit pragma found in the first 8 lines (including the `unit-test` and
   `integration-test` sub-role pragmas);
2. configured `[roles]` path globs (including `[roles.unit-test]` and
   `[roles.integration-test]`), with the most specific matching glob winning;
3. built-in path heuristics (including the test-rooted `unit-test` /
   `integration-test` path heuristics `tests/unit/**`, `test/unit/**`,
   `tests/integration/**`, `test/integration/**`);
4. default `app`.

The current pragma scanner SHALL accept line comments beginning with `#` or
`//`, and block-comment openings beginning with `/*`.

#### Scenario: Pragma wins over path glob
- **WHEN** a file declares an explicit Pretender role pragma and also matches a configured role glob
- **THEN** the pragma role is assigned

#### Scenario: Most specific glob wins
- **WHEN** a file matches both `tests/**` and `tests/manual/**`
- **THEN** the more specific glob is assigned

#### Scenario: Heuristic role is used when no pragma or glob applies
- **WHEN** a file path contains `/vendor/` and no pragma or configured glob applies
- **THEN** the file is assigned role `vendor`

#### Scenario: Unit-test heuristic applies under tests/unit
- **WHEN** a file path is `tests/unit/test_foo.py` and no pragma or configured glob applies
- **THEN** the file is assigned role `unit-test`

### Requirement: Check Command

The system SHALL provide `pretender check [paths...]` as the implemented MVP scan command for explicit file and directory inputs. The command SHALL require at least one path, SHALL recursively scan directories, and SHALL support `--format human|json|sarif`, `--output <path>`, `--mode guidance|tiered|gate`, `--test-report <path>` (analyse a JUnit XML test report for duration findings), and `--execute` (run `[execute] test_cmd` to produce the report before analysing it).

For the current MVP, `--staged`, `--diff-only`, and `--diff-base <ref>` are reserved CLI flags and SHALL exit with code `2` and a `not yet implemented` message when used.

Mode behavior in the current MVP is:
- `guidance`: always exits `0`
- `tiered`: always exits `0` after annotating yellow/red findings
- `gate`: exits non-zero when any file or unit threshold is violated, including duration threshold violations

Every successful `pretender check` run SHALL persist the report cache used by `pretender report`, regardless of output format. `--test-report` and `--execute` MAY be combined with structural analysis of the same `paths`; duration findings and structural findings SHALL both appear in the single resulting report.

#### Scenario: Human check succeeds on clean file
- **WHEN** `pretender check path/to/file.py` runs on a file with no threshold violations
- **THEN** the command exits with code `0`

#### Scenario: Guidance mode does not fail violating file
- **WHEN** `pretender check path/to/file.py --mode guidance` runs on a file with threshold violations
- **THEN** the command exits with code `0` after reporting the violations

#### Scenario: Reserved staged flag is rejected
- **WHEN** `pretender check path/to/file.py --staged` is run
- **THEN** the command exits with code `2` and reports that the feature is not yet implemented

#### Scenario: SARIF output is emitted
- **WHEN** `pretender check path/to/file.py --format sarif` is run
- **THEN** stdout contains valid SARIF 2.1.0 JSON for the analysed findings

#### Scenario: Test report flag analyses durations
- **WHEN** `pretender check path/to/file.py --test-report results.xml` is run with a JUnit report containing an over-threshold testcase
- **THEN** the report includes a duration finding for that testcase

#### Scenario: Execute flag runs configured test command
- **WHEN** `pretender check path/to/file.py --execute` is run with `[execute] test_cmd` and `test_report_path` configured
- **THEN** the system executes `test_cmd`, reads `test_report_path`, and includes the resulting duration findings in the report
