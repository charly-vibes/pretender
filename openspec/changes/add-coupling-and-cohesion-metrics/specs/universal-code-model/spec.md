## MODIFIED Requirements

### Requirement: Import Resolution

The system SHALL represent each successfully parsed source file as one `Module` containing path, language, whole-file span, total/code/comment line counts, code units, and imports. The system SHALL populate `Module.imports` for all supported languages using language-specific AST capture patterns during parsing. Files with role `generated` or `vendor` SHALL be excluded from import resolution (their imports are not populated).

#### Scenario: Rust imports populated
- **WHEN** a Rust source file with `use std::collections::HashMap;` is parsed
- **THEN** `Module.imports` contains one entry with module `"std::collections::HashMap"`

#### Scenario: Python imports populated
- **WHEN** a Python source file with `from pathlib import Path` is parsed
- **THEN** `Module.imports` contains one entry with module `"pathlib"` and name `"Path"`

#### Scenario: All supported languages populate imports
- **WHEN** a source file in any supported language with import statements is parsed
- **THEN** `Module.imports` contains the corresponding import entries