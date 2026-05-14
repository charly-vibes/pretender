## MODIFIED Requirements

### Requirement: Versioned Smell-Weight Catalog Format

The system SHALL define a versioned smell-weight catalog format. A catalog file SHALL contain a `version` integer (currently `1`) and a `[[patterns]]` table array. Each pattern entry SHALL have: `capture` (string, glob-matched against `CallSite.callee`), `weight` (f64, must be > 0), `component` (one of `"A"`, `"B"`, `"C"`), and `rationale` (string, human-readable explanation).

```toml
version = 1

[[patterns]]
capture   = "@call.eval"
weight    = 2.0
component = "C"
rationale = "eval executes arbitrary code at runtime; high conditional complexity"

[[patterns]]
capture   = "@call.global_state"
weight    = 1.5
component = "B"
rationale = "global state mutation increases branching complexity indirectly"
```

#### Scenario: Valid catalog parses successfully
- **WHEN** a catalog TOML file with `version = 1` and at least one `[[patterns]]` entry is loaded
- **THEN** all patterns are available for weight resolution without error

#### Scenario: Invalid weight rejected
- **WHEN** a catalog entry has `weight = 0` or a negative value
- **THEN** the system SHALL reject the catalog with a descriptive error

### Requirement: Smell-Weight Resolution

The system SHALL resolve `smell_weight` for a `CallSite` by glob-matching `CallSite.callee` against catalog `capture` patterns in order. The first match determines the weight and component. If no pattern matches, `weight = 1.0` SHALL be used and the component SHALL default to `C`.

#### Scenario: First matching pattern wins
- **WHEN** two patterns match the same callee and the first has weight 2.0
- **THEN** weight 2.0 is applied

#### Scenario: Unmatched callee uses default weight
- **WHEN** a `CallSite.callee` matches no catalog pattern
- **THEN** `smell_weight = 1.0` is used

#### Scenario: Component routing for matched pattern
- **WHEN** a pattern with `component = "B"` matches a call site
- **THEN** that call's weight contributes to the B component of the ABC vector, not C

### Requirement: Shipped Catalog per Language

The system SHALL ship one built-in catalog per supported language, embedded in the binary. The built-in catalog SHALL be used when no user-supplied override file exists at `~/.config/pretender/smell-weights/<language>.toml`.

#### Scenario: Python built-in catalog active by default
- **WHEN** pretender runs on Python files with no user override for Python
- **THEN** `@call.eval` captures resolve to weight 2.0, component C

#### Scenario: Missing language catalog uses empty catalog
- **WHEN** no built-in catalog exists for a plugin language and no user file is present
- **THEN** all call sites in that language use `weight = 1.0`

### Requirement: User Catalog Override

The system SHALL support user-supplied override files at `~/.config/pretender/smell-weights/<language>.toml`. If this file exists, it SHALL shadow the shipped catalog entirely for that language.

#### Scenario: User override replaces built-in weights
- **WHEN** `~/.config/pretender/smell-weights/python.toml` exists with a different weight for `@call.eval`
- **THEN** the user-specified weight is used, not the shipped default
