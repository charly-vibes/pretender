# pretender

Pretender is a Rust CLI for structural code-quality checks across multiple languages.

## Documentation

- [Getting started](docs/getting-started.md)
- [Configuration reference](docs/configuration.md)
- [Mutation testing](docs/mutation.md)
- [Writing plugins](docs/plugins.md)
- [Language support](languages/)

## Commands

| Command | Description |
|---------|-------------|
| `pretender init` | Interactive wizard: write `pretender.toml`, install hook, generate CI |
| `pretender check <paths...>` | Fast pass/fail scan against configured thresholds |
| `pretender complexity <path>` | Show cyclomatic complexity per function, sorted worst-first |
| `pretender report` | Render cached last check as human, markdown, or HTML |
| `pretender duplication <paths...>` | Structural clone detection via AST subtree hashing |
| `pretender mutation <paths...>` | Mutation testing wrapper (mutmut / cargo-mutants / Stryker / PIT) |
| `pretender hooks install\|uninstall` | Install or remove the pre-commit hook |
| `pretender ci generate github` | Emit `.github/workflows/pretender.yml` |
| `pretender explain <metric>` | Print definition, threshold, and citation for a metric |

## Check flags

```
pretender check <paths...> [flags]

  --format human|json|sarif     Output format (default: human)
  --output <path>               Write report to file instead of stdout
  --mode guidance|tiered|gate   Override pretender.toml mode
  --staged                      Check only git-staged files
  --diff-only                   Check only files changed vs --diff-base
  --diff-base <ref>             Base ref for --diff-only (default: origin/main)
```

## Metrics

| Metric | Formula | Default threshold |
|--------|---------|-------------------|
| `cyclomatic` | 1 + decision points | 10 |
| `cognitive` | Nesting-weighted branch sum | 15 |
| `abc` | √(A²+B²+C²) | 30 |
| `function_lines` | end_line − start_line + 1 | 40 |
| `file_lines` | Total lines in file | 400 |
| `nesting_max` | Maximum control-flow depth | 3 |
| `params` | Formal parameter count | 4 |
| `min_assertions` | Assertions per test function | 1 (test role) |

Run `pretender explain <metric>` for full definition and citation.

## Languages

Python, Rust, Go, JavaScript, TypeScript, Java, Ruby, C, C++

## Configuration

```toml
[pretender]
mode = "tiered"          # guidance | tiered | gate
languages = ["auto"]
exclude = ["vendor/**", "node_modules/**", "**/*_generated.*"]

[thresholds]
cyclomatic_max = 10
cognitive_max = 15
function_lines_max = 40
file_lines_max = 400
nesting_max = 3
params_max = 4
abc_max = 30

[thresholds.test]
cyclomatic_max = 3
cognitive_max = 5
min_assertions = 1

[thresholds.library]
exported_cyclomatic_max = 8
exported_params_max = 3
exported_lines_max = 30

[bands]
cyclomatic = { green = 10, yellow = 15, red = 20 }
cognitive  = { green = 15, yellow = 25, red = 40 }
```

## Development

```bash
just build
just test
just lint
just ci          # full verification pipeline
just preflight   # pre-PR checks
```
