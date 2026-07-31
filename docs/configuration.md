# Configuration Reference

`pretender.toml` is placed at the root of your repository. All sections and
keys are optional — omitted keys fall back to the defaults shown here.

---

## Recommended baseline for Rust CLI repos

For charly-vibes suite repos (Rust CLI tools like pretender, wai, dont,
espectacular, vampiro, testaruda, crua, livin), we recommend pinning roles
and overriding the default thresholds to match a typical Rust project layout:

| Role | Paths | Notes |
|------|-------|-------|
| `test` | `tests/**`, `**/*_test.*`, `spec/**` | Default — already matches |
| `unit-test` | `tests/unit/**` | Explicit sub-role for fast unit tests |
| `integration-test` | `tests/integration/**` | Explicit sub-role for slower integration |
| `script` | `scripts/**`, `examples/**` | Default — already matches |
| `generated` | `**/*_generated.*` | Default — pb.go, protobuf, etc. |

### Why pin roles explicitly

Without a `pretender.toml`, pretender uses library defaults: all files get the
`app` role, tests are detected via heuristics only, and every role's paths use
convention over configuration. This works, but has two downsides:

1. **No sub-role awareness** — `tests/unit/` and `tests/integration/` are not
differentiated, so duration thresholds (`duration_max_ms`) never activate.
2. **No version-controlled baseline** — new team members or CI environments
must guess which role layout your project uses.

Pinning roles in `pretender.toml` resolves both issues and makes the config
part of the reviewed, committed project state.

### Baseline threshold values

These values are copied from `templates/pretender.toml.example` and represent
the recommended starting point for Rust CLI repos:

```toml
[thresholds]
cyclomatic_max        = 10
cognitive_max         = 15
function_lines_max    = 40
file_lines_max        = 400
nesting_max           = 3
params_max            = 4
abc_max               = 30
duplication_pct_max   = 5
mi_min                = 20
coverage_line_min     = 80
coverage_branch_min   = 70
mutation_min          = 60

[thresholds.test]
cyclomatic_max      = 3
cognitive_max       = 5
function_lines_max  = 80
nesting_max         = 2
params_max          = 2
duplication_pct_max = 30
min_assertions      = 1

[thresholds.library]
exported_params_max       = 3
exported_cyclomatic_max   = 8
exported_lines_max        = 30
require_docstring         = true

[thresholds.unit-test]
duration_max_ms = 100

[thresholds.integration-test]
duration_max_ms = 2000
```

### Doctor check for config coverage

`pretender doctor` includes a **Hooks vs config** check that warns when a
pre-commit hook is installed but no `pretender.toml` exists. This catches the
common onboarding failure where automation runs but with library defaults
that may not match the project's quality targets.

### Deploying across the suite

Each suite repo should commit its own `pretender.toml` (start from the
template at `templates/pretender.toml.example`) and adjust thresholds as
needed. The canonical template lives in the pretender repo and serves as the
upstream source of truth for all suite repos.

---

## `[pretender]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `mode` | string | `"tiered"` | Check behaviour: `"guidance"` (hints only, never fails), `"tiered"` (failures scale with severity), `"gate"` (any violation fails) |
| `languages` | array of strings | `["auto"]` | Languages to analyse. `"auto"` detects from file extensions. Explicit values: `"python"`, `"rust"`, `"go"`, `"javascript"`, `"typescript"`, `"java"`, `"ruby"`, `"c"`, `"cpp"` |
| `exclude` | array of glob strings | `["vendor/**", "node_modules/**", "**/*_generated.*"]` | Path globs to skip during analysis |

---

## `[thresholds]`

App-level metric limits applied to files assigned the `app` role (the default
for files not matched by any other role).

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `cyclomatic_max` | integer | `10` | Maximum cyclomatic complexity per function |
| `cognitive_max` | integer | `15` | Maximum cognitive complexity per function |
| `function_lines_max` | integer | `40` | Maximum lines per function |
| `file_lines_max` | integer | `400` | Maximum lines per file |
| `nesting_max` | integer | `3` | Maximum control-flow nesting depth |
| `params_max` | integer | `4` | Maximum formal parameters per function |
| `abc_max` | integer | `30` | Maximum ABC score (√(A²+B²+C²)) per function |
| `duplication_pct_max` | integer | `5` | Maximum structural duplication percentage (0–100) |
| `mi_min` | integer | `20` | Minimum Maintainability Index per file |
| `coverage_line_min` | integer | `80` | Minimum line coverage percentage (0–100) |
| `coverage_branch_min` | integer | `70` | Minimum branch coverage percentage (0–100) |
| `mutation_min` | integer | `60` | Minimum mutation score percentage (0–100) |

### `[thresholds.unit-test]`

Overrides for files assigned the `unit-test` sub-role. Inherits all `[thresholds.test]`
values and overlays the duration threshold.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `duration_max_ms` | integer | `0` | Maximum allowed test duration in milliseconds. `0` disables the check.

### `[thresholds.integration-test]`

Overrides for files assigned the `integration-test` sub-role. Inherits all `[thresholds.test]`
values and overlays the duration threshold.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `duration_max_ms` | integer | `0` | Maximum allowed test duration in milliseconds. `0` disables the check.

### `[thresholds.test]`

Overrides for files assigned the `test` role.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `cyclomatic_max` | integer | `3` | Cyclomatic limit for test functions |
| `cognitive_max` | integer | `5` | Cognitive limit for test functions |
| `function_lines_max` | integer | `80` | Line limit for test functions |
| `nesting_max` | integer | `2` | Nesting limit for test functions |
| `params_max` | integer | `2` | Parameter limit for test functions |
| `duplication_pct_max` | integer | `30` | Duplication tolerance in test files |
| `min_assertions` | integer or null | `1` | Minimum assertion calls per test function; `null` disables |

### `[thresholds.library]`

Overrides for files assigned the `library` role.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `exported_params_max` | integer | `3` | Parameter limit on exported/public functions |
| `exported_cyclomatic_max` | integer | `8` | Cyclomatic limit on exported/public functions |
| `exported_lines_max` | integer | `30` | Line limit on exported/public functions |
| `require_docstring` | boolean | `true` | Require a docstring on every exported/public function |

### `[thresholds.script]`

Overrides for files assigned the `script` role.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `function_lines_max` | integer | `100` | Line limit per function |
| `file_lines_max` | integer | `300` | Line limit per file |

---

## `[bands]`

Colour bands used when `mode = "tiered"` to map raw metric values to severity
levels. Each band specifies thresholds for green (acceptable), yellow (warning),
and red (violation). Values must satisfy `green <= yellow <= red`.

Each band (`cyclomatic`, `cognitive`) is **optional**. When the `[bands]`
section is **entirely absent** from `pretender.toml`, both bands are active with
the defaults shown below. However, once you add a `[bands]` section, any band
key you omit is **disabled** (set to `None`) — it does not fall back to its
default. Always specify both bands together when overriding either one.

```toml
# Inline form
[bands]
cyclomatic = { green = 10, yellow = 15, red = 20 }
cognitive  = { green = 15, yellow = 25, red = 40 }

# Block form (equivalent)
[bands.cyclomatic]
green  = 10
yellow = 15
red    = 20

[bands.cognitive]
green  = 15
yellow = 25
red    = 40
```

### `[bands.cyclomatic]`

Default (when `[bands]` is absent): `{ green = 10, yellow = 15, red = 20 }`

| Key | Type | Description |
|-----|------|-------------|
| `green` | integer | Cyclomatic complexity at or below this is green |
| `yellow` | integer | At or below this is yellow |
| `red` | integer | Above this is red |

### `[bands.cognitive]`

Default (when `[bands]` is absent): `{ green = 15, yellow = 25, red = 40 }`

| Key | Type | Description |
|-----|------|-------------|
| `green` | integer | Cognitive complexity at or below this is green |
| `yellow` | integer | At or below this is yellow |
| `red` | integer | Above this is red |

---

## `[scope]`

Controls which files are analysed during `pretender check`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `diff_only` | boolean | `true` | Written by `pretender init` to record your project preference. Setting this to `true` does **not** automatically enable diff filtering — you must pass `--diff-only` on the CLI (or CI step) to activate it. When active, only files changed relative to `diff_base` are checked |
| `diff_base` | string | `"origin/main"` | Git ref used as the comparison base for `--diff-only` |

---

## `[execute]`

Optional shell commands pretender can run to collect coverage and mutation data.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | boolean | `false` | Run the coverage and mutation commands automatically during `pretender check` |
| `coverage_cmd` | string or null | `null` | Shell command that produces a coverage report. pretender checks the exit code; non-zero is treated as a coverage failure |
| `mutation_cmd` | string or null | `null` | Shell command that runs mutation testing. pretender checks the exit code; use `--score-min` for threshold control instead |
| `test_cmd` | string or null | `null` | Shell command that runs the test suite and produces a JUnit XML report (used with `--execute`) |
| `test_report_path` | string or null | `null` | Path to the JUnit XML report produced by `test_cmd` (relative to repo root) |
| `test_timeout_s` | integer | `600` | Timeout in seconds for `test_cmd`; on timeout pretender emits a `test-cmd-timeout` error |
| `test_time_unit` | string | `"seconds"` | Unit of the JUnit `time` attribute: `"seconds"` (XSD-conformant) or `"milliseconds"` (non-conformant runners) |

---

## `[plugins]`

Controls which built-in language and metric plugins are active.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `languages` | array of strings | `["python","javascript","typescript","go","rust"]` | Language plugins to load |
| `metrics` | array of strings | `["eslint","ruff","clippy"]` | Built-in metric tool plugins to load |

External metric plugins are always loaded in addition to this list from the
first directory found in this order:

1. `$PRETENDER_METRICS_DIR` (if set)
2. `$XDG_CONFIG_HOME/pretender/metrics/` (if `XDG_CONFIG_HOME` is set)
3. `~/.config/pretender/metrics/` (fallback)

See [Writing plugins](plugins.md).

---

## `[output]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `formats` | array of strings | `["human","sarif"]` | Output formats to emit. Valid values: `"human"`, `"json"`, `"sarif"`, `"junit"`, `"markdown"`. At least one value is required |
| `sarif_path` | string | `"pretender.sarif"` | File path for SARIF output when `"sarif"` is in `formats` |

---

## `[roles.*]`

Role matchers assign a role to files based on path globs. The resolved role
determines which `[thresholds.*]` block applies.

Each role section has a single key:

| Key | Type | Description |
|-----|------|-------------|
| `paths` | array of glob strings | Files matching any glob are assigned this role |
| `classname_root` | string | For `[roles.test]` only: directory to search when resolving JUnit `classname` to a source file (default `"tests"`) |

Available roles and their defaults:

| Role | Default paths |
|------|--------------|
| `test` | `["tests/**", "**/*_test.*", "spec/**"]` |
| `unit-test` | `["tests/unit/**"]` |
| `integration-test` | `["tests/integration/**"]` |
| `library` | `["pkg/**", "lib/**"]` |
| `script` | `["scripts/**", "examples/**"]` |
| `generated` | `["**/*.pb.go", "**/*_generated.*"]` |
| `vendor` | `["vendor/**", "node_modules/**"]` |

Example override:

```toml
[roles.test]
paths = ["tests/**", "**/*.spec.*", "**/__tests__/**"]
```

### Role detection order

pretender assigns the first role that matches, checked in this priority order:

1. **Pragma** — a comment on any line of the file: `# pretender: role=<name>` or `// pretender: role=<name>`
2. **Configured glob** — the first `[roles.*]` section whose glob matches the file path
3. **File-name heuristic** — built-in patterns (e.g. `_test.` suffix → test, `_generated.` → generated)
4. **Default** — `app`

---

## Test-duration check

pretender can analyse per-test timing data from a JUnit XML report and flag
tests that exceed role-appropriate duration budgets. This is the first
**dynamic** check in pretender — it consumes a runtime artifact rather than
source text.

### Activation

The check activates when you pass `--test-report <path>` to `pretender check`.
You can also configure `[execute] test_cmd` and use `--execute` to have
pretender run the test suite for you:

```sh
# Analyse an existing report
pretender check --test-report target/junit-report.xml

# Run tests first, then analyse
pretender check --execute
```

### Thresholds

Set duration limits per test sub-role:

```toml
[thresholds.unit-test]
duration_max_ms = 100       # unit tests should be fast

[thresholds.integration-test]
duration_max_ms = 2000      # integration tests can be slower
```

A finding is emitted when `observed_ms > duration_max_ms` (strict comparison,
so a test exactly at the threshold does not fail). A threshold of `0`
(the default) disables the check.

### Sub-role detection

pretender detects `unit-test` and `integration-test` sub-roles using the same
resolution order as other roles:

1. **Pragma** — `# pretender: role=unit-test` or `// pretender: role=unit`
2. **Configured glob** — `[roles.unit-test]` with custom `paths`
3. **Heuristic** — `tests/unit/**` and `test/unit/**` → `unit-test`;
   `tests/integration/**` and `test/integration/**` → `integration-test`
4. **Default** — `test` (no duration threshold)

Sub-roles inherit all `[thresholds.test]` values and overlay only
`duration_max_ms`.

### How timing data is read

pretender parses standard JUnit XML (`<testsuite>` / `<testcase time="...">`).
The `time` attribute is interpreted as seconds by default (XSD-conformant);
set `[execute] test_time_unit = "milliseconds"` for non-conformant runners.
Rounding uses round-half-up to the nearest millisecond.

### JUnit emission incantations per runner

| Runner | Command |
|--------|---------|
| pytest | `pytest --junitxml=report.xml` |
| cargo test | `cargo2junit > report.xml` (requires `cargo2junit` crate) |
| Maven | `mvn surefire-report:report` (produces `target/surefire-reports/*.xml`) |
| Gradle | Built-in: `build/reports/tests/test/*.xml` |
| Jest | `jest --junitOutput=report.xml` (requires `jest-junit` reporter) |

### CLI flags

| Flag | Description |
|------|-------------|
| `--test-report <path>` | Path to a JUnit XML report for duration analysis |
| `--execute` | Run `[execute] test_cmd` before analysing durations |

### Output

Findings appear in all output formats:

- **Human:** `unit-test test_addition tests/unit/test_math.py: 100ms > 50ms`
- **JSON:** `test_findings` top-level array with `test_name`, `file`, `role`,
  `observed_ms`, `threshold_ms`
- **SARIF:** `pretender/test-duration` rule id, `Warning` level

In `gate` mode, any duration finding causes a non-zero exit code.

pretender maintains a local event log that surfaces recurring problem areas
across runs — useful for calibrating thresholds and identifying structural
debt before it compounds.

pretender tracks every violation it reports in `.pretender/history/events.jsonl`
at the root of your repository. Each line is a JSON object (a `ViolationEvent`)
recording the file fingerprint, rule key, role, area, run ID, and Unix timestamp.

Events older than **90 days** are pruned automatically on each run.

From the event log pretender computes two summaries shown at the end of
`pretender check`:

- **Hotspots** — the 10 files with the highest total violation count across at
  least two distinct days. A file that repeatedly triggers is a structural
  problem, not a fluke.
- **Patterns** — the 10 (rule, role, area) combinations that recur most often
  across the most files. Use these to calibrate thresholds or identify
  conventions your team hasn't codified yet.

The `.pretender/` directory should be committed so the feedback loop survives
across machines and CI runs. Note: `events.jsonl` records file paths and
fingerprints — review the file before committing if your repository contains
sensitive filenames.
