## ADDED Requirements

### Requirement: Project Assertion Patterns Configuration

The system SHALL support an optional `[assertions]` table in `pretender.toml` with a `patterns` key. `patterns` SHALL be an array of glob strings matched against `CallSite.callee`. When `[assertions]` is present and `patterns` is non-empty, the project patterns SHALL be merged with language-default assertion patterns. When `patterns = []` is specified, language-default patterns SHALL be suppressed and no call sites are counted as assertions from config.

```toml
[assertions]
patterns = ["expect*", "assert*", "verify*", "check*"]
```

#### Scenario: Project pattern extends language defaults
- **WHEN** `[assertions] patterns = ["expectThat"]` is configured and a test file calls `expectThat(x)`
- **THEN** that call is counted as an assertion, satisfying `min_assertions = 1`

#### Scenario: Language defaults remain active without [assertions]
- **WHEN** no `[assertions]` table is present in `pretender.toml`
- **THEN** assertion detection uses language-default `@assert.*` captures unchanged

#### Scenario: Empty patterns array disables language defaults
- **WHEN** `[assertions] patterns = []` is configured
- **THEN** no call sites are counted as assertions even if they match language-default patterns

#### Scenario: Glob pattern matching
- **WHEN** `patterns = ["assert*"]` is configured and a test file calls `assertEquals(a, b)`
- **THEN** that call is counted as an assertion

#### Scenario: Assertion patterns apply only to test role
- **WHEN** `[assertions] patterns = ["verify*"]` is configured and an `app`-role file calls `verifyData(x)`
- **THEN** that call is NOT counted as an assertion (role is not `test`)
