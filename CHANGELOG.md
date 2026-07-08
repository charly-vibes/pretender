# Changelog

All notable changes to Pretender are documented here.

## [Unreleased]

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
