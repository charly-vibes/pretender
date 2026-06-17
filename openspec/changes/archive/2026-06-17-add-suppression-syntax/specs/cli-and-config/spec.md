## ADDED Requirements

### Requirement: Suppressions List Command

The system SHALL provide `pretender suppressions list` which scans all tracked source files,
collects every parsed suppression pragma, and prints a table including: file path, function or
module name, suppressed rules, reason text, and expiry date (if set). Expired suppressions MUST
be included in the output with a clear `[EXPIRED]` annotation.

#### Scenario: Active suppressions are listed

- **WHEN** `pretender suppressions list` is run on a repository containing active suppression pragmas
- **THEN** each suppression is printed with its file, scope (function or module), rules, reason, and expiry

#### Scenario: Expired suppressions are annotated

- **WHEN** `pretender suppressions list` is run and some pragmas have passed their `until` date
- **THEN** those entries are included in the output with a visible `[EXPIRED]` annotation

#### Scenario: No suppressions yields empty list

- **WHEN** `pretender suppressions list` is run on a repository with no suppression pragmas
- **THEN** the command prints an informational message indicating no suppressions were found and exits with code 0

---

### Requirement: Shared Pretender Pragma Scanner

Language plugin `plugin.toml` manifests SHALL support a `[pragmas]` table that declares
how Pretender line-comment pragmas are recognized for that language. The table MUST support at minimum
a `comment_prefix` key specifying the line comment token (e.g., `"//"`, `"#"`, `"--"`).
The engine MUST use the shared Pretender pragma scanner and plugin-declared prefix when scanning for both suppression and role pragmas. For role pragmas, the shared scanner MUST preserve the role-detection rules from `update-role-detection`: only line-comment prefixes are valid, only the first 10 lines are scanned, and block-comment forms are ignored.

#### Scenario: Plugin comment prefix is used for pragma detection

- **WHEN** a language plugin declares `[pragmas] comment_prefix = "#"` in `plugin.toml`
- **THEN** the engine recognizes `# pretender: allow[rule] reason="..."` as a valid suppression pragma for that language

#### Scenario: Default comment prefix uses built-in language default

- **WHEN** a language plugin does not declare a `[pragmas]` table
- **THEN** the engine uses the built-in default pragma comment prefix for that plugin's language
