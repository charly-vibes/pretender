# Delta specs for update-app-threshold-defaults

## MODIFIED Requirements

### Requirement: Configuration Schema

The system SHALL read `pretender.toml` with tables for `[pretender]`, `[thresholds]`, role-specific threshold tables such as `[thresholds.test]`, `[bands]`, `[scope]`, `[execute]`, `[plugins]`, `[output]`, and `[roles]`. The `mode` value SHALL be one of `guidance`, `tiered`, or `gate`. The implicit default role SHALL be `app`. Unknown config keys SHALL be ignored.

The built-in default thresholds SHALL be:

| table | field | default |
|---|---|---|
| `[thresholds]` | `cyclomatic_max` | 10 |
| `[thresholds]` | `cognitive_max` | 15 |
| `[thresholds]` | `function_lines_max` | 30 |
| `[thresholds]` | `file_lines_max` | 400 |
| `[thresholds]` | `nesting_max` | 3 |
| `[thresholds]` | `params_max` | 4 |
| `[thresholds]` | `abc_max` | 30 |
| `[thresholds]` | `duplication_pct_max` | 5 |
| `[thresholds]` | `void_mutators_max` | 3 |
| `[thresholds]` | `mut_ratio_max` | 0.3 |
| `[thresholds]` | `unwrap_max` | 5 |
| `[thresholds]` | `bool_cluster_max` | 3 |
| `[thresholds]` | `primitive_param_check` | false |
| `[thresholds]` | `inheritance_depth_max` | 2 |
| `[thresholds.test]` | `cyclomatic_max` | 3 |
| `[thresholds.test]` | `cognitive_max` | 5 |
| `[thresholds.test]` | `function_lines_max` | 80 |
| `[thresholds.test]` | `nesting_max` | 2 |
| `[thresholds.test]` | `params_max` | 2 |
| `[thresholds.test]` | `duplication_pct_max` | 30 |
| `[thresholds.test]` | `min_assertions` | 1 |
| `[thresholds.test]` | `mock_count_max` | 5 |
| `[thresholds.test]` | `unwrap_max` | 5 |
| `[thresholds.test]` | `lazy_cluster_min` | 3 |
| `[thresholds.library]` | `exported_params_max` | 3 |
| `[thresholds.library]` | `exported_cyclomatic_max` | 8 |
| `[thresholds.library]` | `exported_lines_max` | 30 |
| `[thresholds.library]` | `require_docstring` | true |
| `[thresholds.script]` | `function_lines_max` | 100 |
| `[thresholds.script]` | `file_lines_max` | 300 |

#### Scenario: Role-specific thresholds override app defaults
- **WHEN** a file is assigned role `test`
- **THEN** values under `[thresholds.test]` override app threshold defaults for that file

#### Scenario: Built-in defaults match the documented table
- **WHEN** `pretender.toml` omits a threshold field
- **THEN** the built-in default from the table above applies (e.g. `function_lines_max` 30, `unwrap_max` 5, `mock_count_max` 5 in tests)

## ADDED Requirements

### Requirement: Threshold Preset Selection

`pretender init` SHALL offer a threshold-preset choice between `default` and `strict`, both interactively (a prompt after the mode question) and non-interactively via `--preset <default|strict>` with default `default`. An explicit `--preset` on an interactive run SHALL suppress the prompt and use the flag value (mirroring `--mode`). When init writes a fresh `pretender.toml`, it SHALL emit the chosen preset's threshold tables explicitly. Init SHALL NOT modify an existing `pretender.toml`.

#### Scenario: Interactive init asks for preset
- **WHEN** `pretender init` runs interactively and the user answers the preset question with `strict`
- **THEN** the generated `pretender.toml` contains the strict threshold tables (e.g. app `cyclomatic_max = 8`)

#### Scenario: Non-interactive preset flag
- **WHEN** `pretender init --non-interactive --preset strict` is run in a repository without `pretender.toml`
- **THEN** the written config uses the strict tables and init exits with code 0

#### Scenario: Unknown preset is rejected
- **WHEN** `pretender init --preset loose` is run
- **THEN** the command exits non-zero reporting `loose` is not a valid preset

### Requirement: Strict Preset Values

The `strict` preset SHALL use: app `cyclomatic_max` 8, `cognitive_max` 12, `function_lines_max` 25, `nesting_max` 2, `params_max` 3, `abc_max` 25, `duplication_pct_max` 4, `file_lines_max` 300; test `cyclomatic_max` 2, `cognitive_max` 3, `function_lines_max` 60, `nesting_max` 2; enabled-check values identical to the default preset.

#### Scenario: Strict tables are pinned
- **WHEN** the strict preset tables are generated
- **THEN** each value matches the pinned strict table exactly

### Requirement: Strict Preset Gate Hint

Init SHALL print the `pretender check --adopt` / tiered-lease migration hint when the effective mode is `gate` together with the `strict` preset.

#### Scenario: Strict and gate prints migration hint
- **WHEN** init runs with mode `gate` and preset `strict`
- **THEN** init prints the `pretender check --adopt` / tiered-lease hint after writing the config
