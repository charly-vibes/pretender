# CLI and Configuration

**Version:** 0.1.0  
**Status:** Baseline

## CLI Commands

```
pretender [init|check|report|hooks|ci]
pretender [complexity|duplication|mutation]
```
### `pretender init`

Interactive wizard. Writes `pretender.toml`, optionally installs a pre-commit hook and generates a CI workflow.

- `--non-interactive` | `--defaults` — skip prompts, use best-guess defaults.
- `--mode <mode>` — override default mode.

```
$ pretender init
? Mode: guidance | tiered | gate
? Languages: auto-detect ✓
? Install pre-commit hook? Yes
? Generate GitHub Actions workflow? Yes
✓ Wrote pretender.toml
✓ Wrote .git/hooks/pre-commit
✓ Wrote .github/workflows/pretender.yml
```

### `pretender check [paths]`

Fast pass/fail scan against configured thresholds. Used by hooks and CI.

- While the system is in `gate` mode, if any metric exceeds its `*_max` threshold, the system shall exit with a non-zero code.
- While the system is in `tiered` mode, if a metric exceeds the maximum threshold but remains within the yellow band, the system shall print a yellow annotation and exit with code 0.
- In `guidance` mode: exit 0 always, informational output.
- `--staged` — only check git-staged files.
- `--diff-only` — only check files changed relative to `diff_base`.
- `--staged` and `--diff-only` may be combined; the result is the **intersection** — only files that are both staged AND changed relative to `diff_base` are checked (the pre-commit hook uses both flags).
- `--diff-base <ref>` — override `diff_base` from config.
- `--format <fmt>` — `human` (default) | `json` | `sarif` | `junit` | `markdown`.
- `--output <path>` — write output to file instead of stdout.
- **Performance Note:** Expensive checks (like `duplication --cross-file` or `mutation`) are disabled by default in `check` unless explicitly enabled in `pretender.toml`.

...

### `pretender plugins list|add|remove`

Manages language and metric plugins in `~/.config/pretender/`.

- `list` — show installed plugins and their versions.
- `add <url|path>` — install a plugin from a git URL or local path.
- `remove <name>` — uninstall a plugin.
- Top 10 by default; `--top N` to change
- `--threshold <n>` — highlight above this ABC score
- Emits per-unit breakdown: A, B, C components and weighted total

### `pretender duplication [paths]`

Structural clone detection via normalised AST subtree hashing.

- Hashes subtrees of ≥10 nodes (configurable via `--min-nodes`)
- Reports clone pairs with location, size, and similarity score 0–100
- V0: within-file only; V1: cross-file with `--cross-file` flag

### `pretender mutation [paths]`

Mutation testing wrapper. Delegates to per-language tools (Stryker / PIT / mutmut / cargo-mutants).

- `--score-min <n>` — override mutation score threshold (default: 60, matching Stryker's default)
- `--format <fmt>` — `human` (default) | `json`

### `pretender report`

Pretty TUI or HTML report from the last `check` run.

- `--format human|html` — `human` renders in the terminal; `html` writes a static file
- Reads cached results from the last `pretender check` invocation

### `pretender hooks install`

Writes `.git/hooks/pre-commit` (native shim, zero deps) or generates lefthook/pre-commit YAML.

```bash
# Generated shim (default)
#!/usr/bin/env sh
exec pretender check --staged --diff-only
```

### `pretender hooks uninstall`

Removes the hook file(s) previously installed by `pretender hooks install`.

### `pretender ci generate <provider>`

Providers: `github` | `gitlab` | `circle` | `azure` | `generic`

GitHub output uses SARIF upload to `github/codeql-action/upload-sarif` so findings appear inline in PRs.

### `pretender plugins list|add|remove`

Manages language and metric plugins in `~/.config/pretender/`. Flags: V1, details TBD.

### `pretender explain <metric>`

Prints metric definition and threshold citation (McCabe 1976, SonarSource, Google/Microsoft style guides).

## Config Schema (`pretender.toml`)

```toml
[pretender]
mode = "tiered"              # guidance | tiered | gate
languages = ["auto"]         # or explicit list
exclude = ["vendor/**", "node_modules/**", "**/*_generated.*"]

[thresholds]                 # defaults = app role
cyclomatic_max         = 10
cognitive_max          = 15
function_lines_max     = 40
file_lines_max         = 400
nesting_max            = 3
params_max             = 4
duplication_pct_max    = 5
# Maintainability Index (Microsoft 0–100 scale): weighted combination of Halstead volume,
# cyclomatic complexity, and lines of code. ≥20 = green zone. Only computed when
# [execute] enabled = true. Use `pretender explain mi` for formula and citations.
mi_min                 = 20

coverage_line_min      = 80  # only enforced when [execute] enabled = true
coverage_branch_min    = 70
mutation_min           = 60

[bands]                      # tiered mode: values outside _max but inside band = yellow
# Only metrics with explicit [bands] entries use green/yellow/red banding.
# Metrics without a bands entry (nesting, params, function_lines, duplication_pct)
# are binary: pass (≤ threshold) or fail (> threshold).
cyclomatic = { green = 10, yellow = 15, red = 20 }
cognitive  = { green = 15, yellow = 25, red = 40 }

[thresholds.test]
cyclomatic_max     = 3
function_lines_max = 80
nesting_max        = 2
params_max         = 2
cognitive_max      = 5
duplication_pct_max = 30
min_assertions     = 1

[thresholds.library]
exported_params_max     = 3
exported_cyclomatic_max = 8
exported_lines_max      = 30
require_docstring       = true

[thresholds.script]
function_lines_max = 100
file_lines_max     = 300

[scope]
diff_only = true
diff_base = "origin/main"

[execute]
enabled      = false
# coverage_cmd = "your-test-runner --coverage-output=xml"
# mutation_cmd = "your-mutation-tool run"

[plugins]
languages = ["python", "javascript", "typescript", "go", "rust"]
metrics   = ["eslint", "ruff", "clippy"]

[output]
formats    = ["human", "sarif"]
sarif_path = "pretender.sarif"

[roles]
# The `app` role is implicit: any file not matched by entries below is role=app.
test      = { paths = ["tests/**", "**/*_test.*", "spec/**"] }
library   = { paths = ["pkg/**", "lib/**"] }
script    = { paths = ["scripts/**", "examples/**"] }
generated = { paths = ["**/*.pb.go", "**/*_generated.*"] }
vendor    = { paths = ["vendor/**", "node_modules/**"] }
```

## Schema Versioning

Config files do not require a version key. Unknown keys are silently ignored (forward compatible). Breaking changes to key semantics require a new major semver version and are announced in `CHANGELOG.md`. Tools consuming `pretender.toml` should ignore unknown keys rather than erroring.

## Output Formats

| Format | Use |
|--------|-----|
| `human` | Terminal, colored, default |
| `json` | Machine pipelines, custom integrations |
| `sarif` | GitHub Code Scanning, GitLab SAST, IDE diagnostics (OASIS SARIF 2.1.0) |
| `junit` | CI test reporters |
| `markdown` | `$GITHUB_STEP_SUMMARY`, PR comments |

SARIF is the highest-priority format — once valid SARIF is emitted, GitHub PR annotations, IDE squiggles (SARIF viewer extension), and future aggregators work automatically.

## Plugin Manifests

### Language Plugin

```toml
# ~/.config/pretender/languages/elixir/plugin.toml
name         = "elixir"
display_name = "Elixir"
extensions   = [".ex", ".exs"]
shebangs     = ["elixir"]
tree_sitter  = { source = "github:elixir-lang/tree-sitter-elixir", rev = "main" }
query        = "metrics.scm"

[branches]
"@branch.if"      = { cyclomatic = 1, cognitive = 1 }
"@branch.loop"    = { cyclomatic = 1, cognitive = 1 }
"@branch.logical" = { cyclomatic = 1, cognitive = 1 }
"@branch.catch"   = { cyclomatic = 1, cognitive = 1 }

[assertions]
patterns = ["assert", "assert_eq!", "assert_ne!"]

# [smell_weights] — no Elixir-specific smell weights defined yet
```

### Metric Plugin (External Tool Wrapper)

```toml
# ~/.config/pretender/metrics/eslint.toml
name       = "eslint"
applies_to = ["javascript", "typescript"]
command    = "eslint --format json {files}"
parser     = "json"
mapping    = { "errorCount" = "issues.error", "warningCount" = "issues.warn" }
```
