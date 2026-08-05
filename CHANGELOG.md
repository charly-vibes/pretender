# Changelog

All notable changes to Pretender are documented here.

## [0.5.0] — 2026-08-05

### Fixed

- **`complexity` crashes on directory paths** — `pretender complexity .` now
  expands directories via `collect_input_files()` instead of calling
  `fs::read_to_string` on a directory.
- **`mutation` fails on directory paths** — `pretender mutation <dir>` now
  expands directories before language detection, fixing
  "no supported source files found" errors.
- **Nested definitions inflate enclosing-function complexity** — branches inside
  nested functions and closures are no longer counted against the parent in all
  13 non-Python languages. `walk_block()` and `collect_nested_blocks()` now use
  the language-aware `is_nested_definition()` predicate (covers Rust
  `function_item`, Go/Java/C# `method_declaration`, Go `func_literal`, Rust
  `closure_expression`, and lambda kinds).
- **Global `--json` flag ignored by `check`** — `pretender --json check .` now
  emits JSON output, matching `doctor`.
- **Generated CI workflow had wrong GitHub org URL** — `ci generate github` now
  emits `charly-vibes/pretender`, matching the `Cargo.toml` `repository` field.
- **Hooks written to CWD-relative `.git` paths** — `hooks install|uninstall` now
  discover the repository root by walking up from the current directory,
  preventing bogus nested `.git` directories when invoked from a subdirectory.
  Running hooks outside a git repo fails clearly.
- **`-v` short flag unusable after `check` subcommand** — check's local `--verbose`
  (no short form) shadowed the global `-v`. Renamed to `--show-all`.
- **Phantom config thresholds removed** — `mi_min`, `coverage_line_min`,
  `coverage_branch_min`, `mutation_min` were documented as enforced but never
  read by `check`. Removed from the schema and docs (existing configs with these
  keys are silently ignored). Mutation score remains gateable via
  `pretender mutation --score-min N`.
- **`plugins` subcommand marked as not yet implemented** in `--help` output.

### Docs

- **Falsifiable value proposition** added to README, AGENTS.md, and
  `openspec/project.md`.
- **Behavioral guardrails** (Goal Sandwich, prohibitions, escalation triggers,
  Minimal Footprint, drift detection) added to AGENTS.md.
- `docs/getting-started.md` output example matches actual `check` human format.
- `docs/status.md` marks `check --staged` as fully implemented.
- Removed duplicate "Check flags" section from README.
- `docs/mutation.md` `--score-min` reference updated after threshold removal.

## [0.4.0] — 2026-07-31

### Added

- **Global `--verbose`/`--quiet` flags** — progressive disclosure via genesis `CliVerbosity` conventions
- **Global `--json`/`--human` flags** — format selection with TTY auto-detect via genesis `CliFormat`
- **`pretender completions <shell>`** — shell completion generation via genesis `cli::generate_completions`
- **`pretender doctor --json`** — JSON output for diagnostics (replaces `--format json`)
- **`pretender feedback --from-last-error`** — auto-populates issue body from last error scratch
- **`--version --json`** — structured version output in genesis envelope format
- **Genesis discovery manifest** — `pretender init` registers in `.genesis/tools.toml`
- **Status contributor** — pretender reports health status for cross-tool dashboard
- **Compile tests for all 14 genesis modules** — cli, scaffold, discovery, fixture, aix
  coverage merged into `genesis_compile_test.rs` (now 12 tests)

### Changed

- **Adopted genesis v0.4.0 modules**: `doctor` (DoctorCheck trait + DoctorRunner),
  `feedback` (handle_feedback), `cli` (completions + version-json), `scaffold`
  (init), `status` (StatusContributor), `discovery` (manifest registration),
  `fixture` (Fixture/FixtureBuilder), `aix` (agents_block helper)
- **Doctor checks rewritten** using `DoctorCheck` trait — 6 diagnostic checks
  with proper skip/dependency logic via `Severity::Warning`
- **Doctor JSON output** now uses genesis envelope shape (`data.checks` array)
- **Feedback subcommand** delegates to genesis `handle_feedback()` (unified
  across all charly-vibes tools)
- **Init command** uses genesis `Scaffold` builder and registers in tool manifest

### Removed

- Custom `DoctorFormat` enum (replaced by genesis `OutputFormat`)
- Custom `build_feedback_body()` (replaced by genesis `handle_feedback`)
- Redundant `create_issue`/`redactor` imports (handled by genesis)

## [0.3.1] — 2026-07-28

### Fixed

- **Crash on Unix sockets in directory walk** — `pretender check .` no longer
  panics when `.git/fsmonitor--daemon.ipc` (or other socket/FIFO entries) is
  encountered during tree traversal. Non-regular, non-directory filesystem
  entries are now silently skipped.
- **Stale version assertion in test** — `test_version_flag_works` was checking
  for `"0.2."` after the crate was bumped to `0.3.0`. Updated to `"0.3."`.

## [0.3.0] — 2026-07-18

### Added

- **Feedback loop extended to all metrics** — `pretender check` now emits
  violation events for ALL threshold categories (cyclomatic, params, nesting,
  function_lines, abc, duplication, min_assertions), not just cognitive.
  History is persisted to `.pretender/history/` with 90-day retention.
- **JSON output includes history** — `pretender check --format json` now
  includes `history` field with hotspots and recurring patterns.
- **Default path** — `pretender check`, `complexity`, `duplication` now
  default to scanning the current directory when no path is given.
- **Agent integration template** — `docs/agent-integration.md` provides
  copyable CLAUDE.md/AGENTS.md snippets for project discoverability.
- **wai way integration** — `check_pretender()` added to `wai way`;
  `wai way code-quality` topic guide covers installation, configuration,
  hooks, and CI.
- **Research docs** — `docs/research/agent-adoption.md` and related docs
  document pretender adoption analysis.

### Changed

- Bumped MSRV-compatible dependency versions.
- Internal: HistorySummary, HotspotSummary, PatternSummary now implement Clone.

### Fixed

- CLI no longer requires a path argument for check, complexity, duplication.
- Clippy lints (single_element_loop).

## [0.2.0] — 2026-07-08

### Added

#### Language plugins
- **R** — language plugin (tree-sitter-r), extensions `.r`, `.R`
- **Julia** — language plugin (tree-sitter-julia), extension `.jl`
- **C#** — language plugin (tree-sitter-c-sharp), extension `.cs`;
  pinned to compatible ABI version
- **Clojure** — language plugin (tree-sitter-clojure), extensions
  `.clj`, `.cljs`, `.cljc`, `.edn`; homoiconic syntax evaluated via
  symbol-text matching with `#match?` predicates

#### Diagnostics
- `pretender doctor` — 6 built-in health checks for config, hooks,
  plugin manifests, and doctor exit codes; `--format human|json`
- **Unsupported-language warning** — `pretender check` warns on stderr
  when explicit paths contain no files with supported language extensions

#### Engine improvements
- **tree-sitter upgrade** — 0.23 → 0.25 (v0.23.2 → v0.25.10);
  `QueryCursor::matches` adapted to streaming-iterator API;
  all grammar crates updated to latest compatible versions
- **Body-as-branch handling** — function body checked against capture
  map before walking children, fixing languages where the entire
  body is a single branch form (e.g. Clojure `(if ...)` as defn body)

#### Test infrastructure
- Integration tests for all new language plugins
- Updated unsupported-language test to use `.hs` (Haskell fixture)

### Changed

- `tree-sitter` dependency relaxed from `"0.23"` to `"0.25"`
- `tree-sitter-c-sharp` unpinned from `=0.23.0` to `"0.23"`
- Added `streaming-iterator = "0.1"` dependency
- Added `tree-sitter-clojure = "0.1"` dependency
- `Language` enum extended with `R`, `Julia`, `CSharp`, `Clojure`

## [0.1.0] — MVP

### Added

#### Core CLI
- `pretender init` — interactive wizard; writes `pretender.toml`, installs hook, generates CI workflow
- `pretender check <paths...>` — fast pass/fail scan against configured thresholds
  - `--format human|json|sarif` output formats
  - `--output <path>` write report to file
  - `--mode guidance|tiered|gate` runtime mode override
  - `--staged` check only git-staged files (pre-commit integration)
  - `--diff-only` / `--diff-base <ref>` check only files changed relative to a base ref
- `pretender complexity <path>` — show cyclomatic complexity per function, sorted worst-first
- `pretender report` — render cached last check as `human`, `markdown`, or `html`
- `pretender duplication <paths...>` — structural clone detection via normalised AST subtree hashing
  - `--min-nodes <n>` minimum subtree size (default 10)
  - `--cross-file` detect clones across files
- `pretender mutation <paths...>` — mutation testing wrapper (mutmut / cargo-mutants / Stryker / PIT)
  - `--score-min <n>` minimum mutation score gate (default 60)
  - `--dry-run` list planned mutation sites without running tests
  - `--format human|json`
- `pretender hooks install|uninstall` — safe pre-commit hook management with Pretender-marker guard
- `pretender ci generate github` — emit `.github/workflows/pretender.yml`
- `pretender explain <metric>` — print definition, formula, default threshold, citation, and improvement tip for any built-in metric

#### Metrics (all languages)
- **Cyclomatic complexity** — 1 + decision points; threshold 10 (McCabe 1976)
- **Cognitive complexity** — nesting-weighted mental effort; threshold 15 (Campbell/SonarSource 2018)
- **ABC score** — √(A²+B²+C²) with per-call smell weights; threshold 30 (Fitzpatrick 1997)
- **Function lines** — line span of a function; threshold 40
- **File lines** — total lines in a file; threshold 400
- **Nesting depth** — maximum control-flow nesting; threshold 3
- **Parameter count** — formal parameters per function; threshold 4
- **Min assertions** — minimum assertions per test function; threshold 1 (test role)
- **Exported surface limits** — tighter cyclomatic (8), params (3), lines (30) for library exported symbols

#### Languages
- Python, Rust, Go, JavaScript, TypeScript, Java, Ruby, C, C++ — all backed by tree-sitter adapters

#### Configuration (`pretender.toml`)
- `[pretender]` — mode, language list, exclude patterns
- `[thresholds]` — per-metric limits with role-specific overrides (`[thresholds.test]`, `[thresholds.library]`, `[thresholds.script]`)
- `[bands]` — yellow/red bands for cyclomatic and cognitive (tiered mode)
- `[scope]` — `diff_base`, `diff_only`
- `[roles.*]` — path-glob overrides per role
- Role detection: pragma → configured glob → file-name heuristic → `app`

#### External plugins
- Plugin runner for ESLint, Ruff, Clippy, staticcheck — reads `~/.config/pretender/metrics/` TOML manifests
- External findings merged into `check` output alongside built-in metric violations

#### History & feedback loop
- `cognitive_max` violations persisted to `.pretender/events.jsonl`; rolling 90-day window
- Hotspot and pattern summaries printed after `check` (human format)

#### Output formats
- Human (coloured terminal, severity bands)
- JSON (structured `CheckReport`)
- SARIF 2.1.0 (GitHub Code Scanning compatible)
- Markdown and HTML report via `pretender report`

### Reserved (not yet implemented)
- `pretender plugins list|add|remove` — tracked in pretender-07m
