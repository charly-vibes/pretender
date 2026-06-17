## ADDED Requirements

### Requirement: pretender explain abc Language Subcommand

The system SHALL support `pretender explain abc --language <lang>`, which prints the active smell-weight table for the given language. The output SHALL include the `capture` pattern, `weight`, `component`, and `rationale` for each entry, clearly marking whether each entry comes from the shipped catalog or a user override.

#### Scenario: Active weights printed for known language
- **WHEN** `pretender explain abc --language python` is run
- **THEN** output lists all active patterns for Python including at least `@call.eval` with weight 2.0

#### Scenario: Language not recognised
- **WHEN** `pretender explain abc --language cobol` is run and no catalog exists for `cobol`
- **THEN** the command exits non-zero with a message identifying the unknown language

#### Scenario: JSON format output
- **WHEN** `pretender explain abc --language python --format json` is run
- **THEN** output is a JSON array of pattern objects with `capture`, `weight`, `component`, `rationale`, and `source` (one of `"builtin"` or `"user"`) fields

### Requirement: Smell-Weights TOML Override Path

The system SHALL document and honour `~/.config/pretender/smell-weights/<language>.toml` as the user override path for smell-weight catalogs. The file format SHALL match the versioned catalog format defined in the universal-code-model spec.

#### Scenario: Override file discovered automatically
- **WHEN** `~/.config/pretender/smell-weights/rust.toml` exists
- **THEN** `pretender explain abc --language rust` shows source as `"user"` for all patterns in that file
