# Configuration Reference

`pretender.toml` is placed at the root of your repository. All sections and
keys are optional — omitted keys fall back to the defaults shown here.

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
| `diff_only` | boolean | `true` | Stored for reference; does not enable diff filtering by itself. Pass `--diff-only` on the CLI to activate scope filtering. When active, only files changed relative to `diff_base` are checked |
| `diff_base` | string | `"origin/main"` | Git ref used as the comparison base for `--diff-only` |

---

## `[execute]`

Optional shell commands pretender can run to collect coverage and mutation data.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | boolean | `false` | Run the coverage and mutation commands automatically during `pretender check` |
| `coverage_cmd` | string or null | `null` | Shell command that produces a coverage report. pretender checks the exit code; non-zero is treated as a coverage failure |
| `mutation_cmd` | string or null | `null` | Shell command that runs mutation testing. pretender checks the exit code; use `--score-min` for threshold control instead |

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

Available roles and their defaults:

| Role | Default paths |
|------|--------------|
| `test` | `["tests/**", "**/*_test.*", "spec/**"]` |
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

## History and feedback loop

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
