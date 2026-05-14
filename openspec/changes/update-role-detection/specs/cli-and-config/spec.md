## ADDED Requirements

### Requirement: Three-Tier Role Detection

The system SHALL assign a role to each file using exactly three tiers in priority order. **BREAKING**: the heuristics tier is removed and no other role assignment logic exists.

1. **Pragma**: if `// pretender: role=<role>` (or language-appropriate comment prefix) appears within the first 10 lines of the file, that role is assigned
2. **Path glob**: if the file path matches a glob in `[roles]` config, the matching role is assigned; when multiple globs match, the first defined entry in config wins
3. **Default**: the role `app` is assigned

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

#### Scenario: Pragma wins over matching glob
- **WHEN** a file has `// pretender: role=app` and its path matches `tests/**`
- **THEN** the file is assigned role `app`, not `test`

### Requirement: Pragma Comment Syntax

The system SHALL recognise language-appropriate comment prefixes for the role pragma. For C-family languages the prefix is `//`. For Python, Ruby, Shell, and TOML the prefix is `#`. The pragma form is `<prefix> pretender: role=<role>` with no leading whitespace required on the comment prefix.

#### Scenario: Python pragma recognised
- **WHEN** a `.py` file contains `# pretender: role=script` within the first 10 lines
- **THEN** the file is assigned role `script`

#### Scenario: Unknown prefix not treated as pragma
- **WHEN** a file contains `/* pretender: role=test */` (block comment form)
- **THEN** this is not treated as a valid pragma; glob/default tiers apply

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
