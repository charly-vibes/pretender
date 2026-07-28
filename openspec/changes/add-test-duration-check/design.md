# Design: Test-duration check

## Context

Pretender's entire evaluation pipeline today is **static**: tree-sitter parses
source, metrics are pure functions over the universal code model, and
thresholds are compared against computed values. A test-duration check is
fundamentally **dynamic** — it requires observed runtime data that no AST can
provide. This is the first check in pretender whose input is a runtime
artifact rather than source text.

The `[execute]` config table exists with `coverage_cmd` and `mutation_cmd`
fields, but these are **config-only and never executed today** — the
reserved `mutation` command dispatches to hardcoded per-language tools
(`cargo mutants` / `mutmut` / `stryker`) and never reads `[execute]`. So
test-duration is the **inaugural consumer** of `[execute]` command
execution. This means the run contract (working directory, environment,
timeout, exit-code handling) is authored here rather than inherited from a
prior implementation.

The user confirmed two design decisions:

1. **Timing data source — both**: parse a pre-existing JUnit XML report by
   default (language-agnostic, no per-runner adapters), with an optional
   execute mode that runs `[execute] test_cmd` to produce that report.
2. **Test categorization — both**: detect `unit-test` vs `integration-test`
   sub-roles via pragma → configured glob → heuristic, mirroring the existing
   role resolution order.

## Goals / Non-Goals

- Goals:
  - Gate on per-test elapsed time with role-appropriate budgets.
  - Stay language-agnostic by leaning on JUnit XML as the universal timing
    interchange (every major runner can emit it: `pytest --junitxml`,
    `cargo2junit`, `maven-surefire`, `gradle`, `jest-junit`, etc.).
  - Fit the new check into the existing `check` surface, output formats, and
    `gate`-mode semantics.
- Non-Goals:
  - Do not build per-runner timing-output parsers (stdout scraping). JUnit
    XML is the only ingestion format in this change.
  - Do not run tests in-process or measure timing yourself; defer to the
    runner via `test_cmd`.
  - Do not restructure the universal code model. Durations are not AST-derived.
  - Do not add a `--top`/ranking or historical-trend view in this change.

## Decisions

### Decision 1: JUnit XML as the timing interchange

Parse JUnit XML `<testsuite>` / `<testcase time="...">`. Per the JUnit XSD
the `time` attribute is in seconds (fractional); convert to milliseconds
(u32, round-half-up) for comparison against `duration_max_ms`. Real-world
runners occasionally emit milliseconds or omit `time` entirely; the default
interpretation is seconds (XSD-conformant), overridable via
`[execute] test_time_unit = "seconds" | "milliseconds"` so a
non-conformant report does not silently produce 1000× misreadings.

- Why: single format covers virtually every test runner, avoiding a per-runner
  adapter explosion.
- Alternatives considered:
  - Parse runner-specific stdout (`cargo test --report-time`, pytest
    `--durations`). Rejected as primary path: N adapters, brittle, runner-
    version dependent. Kept as future extension.
  - Define a pretender-native JSON timing schema. Rejected: JUnit already
    exists and is a lowest-common-denominator.

### Decision 2: Mapping a JUnit testcase to a source file / role

JUnit `<testcase>` has `classname` and `name`; many runners also emit a
`file` attribute. Resolution strategy for role assignment:

1. If the testcase has a `file` attribute, resolve the source file directly.
2. Else derive a candidate path from `classname` using a concrete default
   algorithm: replace `.` with `/`, append the detected language extension
   (e.g. `.py`, `.rs`), and search under the configured
   `[roles.test] classname_root` (default `tests`). The first existing file
   wins; the search root is overridable per project.
3. Run the resulting path through normal role detection to get
   `unit-test` / `integration-test` / `test`.
4. Testcases whose `classname` resolves to no existing file are reported
   under the base `test` role with a note rather than dropped.

- Why: keeps duration evaluation reusing the existing role pipeline rather
  than a parallel classifier, and makes the common (no-`file`) path
  normative and testable instead of best-effort.
- Risk: classname→path mapping is imperfect across runners. Mitigation:
  the `classname_root` knob absorbs project layout variance, the `file`
  attribute is preferred when present, and unresolved cases degrade to the
  base `test` role (which has no duration threshold by default), so the
  check degrades safely.

### Decision 3: Sub-roles refine `test`, inherit `test` thresholds

`unit-test` and `integration-test` are new role variants. A file classified
as one of them inherits the `test` thresholds and overlays only the
sub-role-specific fields (`duration_max_ms`). `EffectiveThresholds::for_role`
gains the two variants.

- Why: consistent with how `test` already overrides `app` defaults and how
  `library`/`script` overlay their fields.
- Alternative: a single `test` role with a duration table keyed by sub-role.
  Rejected: it would split role detection from threshold resolution, breaking
  the existing one-role → one-threshold-table invariant.

### Decision 4: `duration_max_ms` is a u32 in milliseconds

Plain integer milliseconds, consistent with the other numeric thresholds
(`cyclomatic_max`, `function_lines_max`, etc.). No duration-string parsing
(`"100ms"`) in this change; a human-friendly string form is a future
enhancement if requested. Conversion from the JUnit `time` attribute uses
round-half-up to the nearest millisecond, and the gate comparison is strict:
a finding is emitted iff `rounded_ms > duration_max_ms` (a test exactly at
threshold does not fail).

- Why: keeps config deserialization trivial and validation rules uniform
  (non-negative u32; `0` means "no limit / disabled"), and makes the
  boundary deterministic.

### Decision 5: Opt-in, additive, non-breaking

The duration check only activates when at least one of these is present:
`--test-report <path>`, `[execute] test_cmd` with `--execute`, or a
configured `duration_max_ms` on a test sub-role. Otherwise `check` is
byte-for-byte the current behavior.

- Why: avoids forcing every existing user to set up JUnit emission.

### Decision 6: Execute-mode command-running contract

Because `test_cmd` is the first `[execute]` field ever executed, this
change defines the run contract:

- **Working directory:** the repository root (where `pretender.toml` lives).
- **Environment:** inherited from the parent process.
- **Timeout:** bounded by `[execute] test_timeout_s` (u32, default `600`).
  On timeout the system emits a single error finding `test-cmd-timeout` and,
  in `gate` mode, exits non-zero. This prevents a test-duration gate from
  itself hanging indefinitely on a stalled suite.
- **Exit code:** a non-zero exit from `test_cmd` is surfaced as an error
  finding (the report may still be partial); the duration check proceeds on
  whatever report `test_report_path` contains.

### Decision 7: Report attachment and exit predicate

Duration findings are per-test-case and frequently reference source files
outside the scanned `paths`, so they do not fit the existing `FileReport` /
`UnitReport` tree. The change adds a top-level `test_findings:
Vec<TestDurationFinding>` to `CheckReport` (serialized with
`skip_serializing_if = Vec::is_empty`), carrying test name, resolved file
(when available), role, observed ms, and threshold ms. `decide_exit_code`
is extended to treat a non-empty `test_findings` as a violation in
`gate` mode, mirroring the existing `file_violations` / `unit.violations`
checks. `report` renders duration findings inline in the findings list
(discriminator `kind = duration`) rather than in a dedicated section.

## Risks / Trade-offs

- **JUnit attribute variance**: runners disagree on `time` precision and
  whether `file` is emitted. Mitigation: tolerant parser, document known
  runners, fall back to `classname` mapping, degrade to base `test` role.
- **Flaky timing**: wall-clock test durations vary run-to-run, risking
  flaky gates. Mitigation (out of scope here, but documented): a future
  `duration_max_ms` could be expressed as a percentile over N runs; this
  change uses a single-run observation. Document the flakiness trade-off in
  user-facing docs.
- **New dependency surface**: adding an XML parser grows the dependency
  tree. Mitigation: pick the smallest viable crate (`quick-xml` hand-rolled
  vs `quick-junit`); gate the dependency behind the new feature only.

## Migration Plan

1. Add config fields with defaults that disable the check (`duration_max_ms
   = 0`, no `test_cmd`). Existing `pretender.toml` files parse unchanged.
2. Document the new tables and the JUnit emission incantation per runner in
   user docs.
3. No data migration; the report cache gains optional duration findings, which
  old `report` consumers ignore if they only read structural findings.

## Open Questions

- Exact crate choice for JUnit parsing (`quick-junit` vs hand-rolled
  `quick-xml`). Defer to the implementation task; both satisfy the spec.
