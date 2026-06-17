## MODIFIED Requirements

### Requirement: Role Detection

The system SHALL assign each file a role from `app`, `library`, `test`, `script`, `generated`, or `vendor` using exactly three tiers in priority order. **BREAKING**: this change modifies the MVP role detector from `pragma → glob → heuristic → default` to a three-tier model with no heuristic fallback.

1. **Pragma**: if a supported line-comment prefix followed by `pretender: role=<role>` appears within the first 10 lines of the file, that role is assigned.
2. **Path glob**: if the file path matches a glob in `[roles]` config, the matching role is assigned; when multiple globs match, the first defined entry in config wins.
3. **Default**: the role `app` is assigned.

Built-in line-comment prefixes are `//` for C-family languages and `#` for Python, Ruby, Shell, and TOML. Block-comment forms such as `/* pretender: role=test */` SHALL NOT be treated as valid role pragmas.

#### Scenario: Pragma in first line assigns role
- **WHEN** a file begins with `// pretender: role=test`
- **THEN** the file is assigned role `test` regardless of its path

#### Scenario: Pragma on line 10 is detected
- **WHEN** `// pretender: role=library` appears exactly on line 10
- **THEN** the file is assigned role `library`

#### Scenario: Pragma on line 11 is ignored
- **WHEN** `// pretender: role=library` appears on line 11 or later
- **THEN** the pragma is not considered; glob and default tiers apply normally

#### Scenario: Path glob assigns role when no pragma
- **WHEN** a file has no pragma and its path matches `tests/**` in `[roles]` config
- **THEN** the file is assigned role `test`

#### Scenario: Default role when no pragma and no glob match
- **WHEN** a file has no pragma and its path matches no configured glob
- **THEN** the file is assigned role `app`

#### Scenario: Matching globs use first-defined entry
- **WHEN** a file matches both `tests/**` and `tests/manual/**`, and `tests/**` is defined first in config
- **THEN** the file is assigned the role from `tests/**`

#### Scenario: Pragma wins over matching glob
- **WHEN** a file has `// pretender: role=app` and its path matches `tests/**`
- **THEN** the file is assigned role `app`, not `test`

#### Scenario: Files that formerly relied on heuristics fall back to app
- **WHEN** a file has no pragma, matches no configured glob, and would previously have received a heuristic role
- **THEN** the file is assigned role `app`

#### Scenario: Python pragma recognised
- **WHEN** a `.py` file contains `# pretender: role=script` within the first 10 lines
- **THEN** the file is assigned role `script`

#### Scenario: Block comment form not treated as pragma
- **WHEN** a file contains `/* pretender: role=test */` (block comment form)
- **THEN** this is not treated as a valid pragma; glob/default tiers apply

## ADDED Requirements

### Requirement: pretender check --explain-roles Flag

The system SHALL support `--explain-roles` on `pretender check`. When set, the system SHALL print one line per scanned file before check results, in the format: `<path>  role=<role>  source=<source>` where `<source>` is one of `pragma`, `glob:<pattern>`, or `default`.

#### Scenario: Explain roles output shows pragma source
- **WHEN** `pretender check --explain-roles` is run on a file with a role pragma
- **THEN** output for that file shows `source=pragma`

#### Scenario: Explain roles output shows glob source with pattern
- **WHEN** `pretender check --explain-roles` is run on a file matched by glob `tests/**`
- **THEN** output for that file shows `source=glob:tests/**`

#### Scenario: Explain roles output shows default source
- **WHEN** `pretender check --explain-roles` is run on a file with no pragma and no glob match
- **THEN** output for that file shows `source=default`
