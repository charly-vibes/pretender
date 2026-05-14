## 1. Pragma Parser

- [ ] 1.1 Implement `parse_suppression_comment(line: &str) -> Option<Suppression>` for `// pretender: allow[...]` syntax
- [ ] 1.2 Validate `reason` is present and non-empty; emit a validation error if missing
- [ ] 1.3 Reject `allow[*]`; emit a validation error listing that wildcard is forbidden
- [ ] 1.4 Parse optional `until="YYYY-MM-DD"` field; validate date format
- [ ] 1.5 Implement expiry check: if `until` is in the past, ignore suppression and emit a warning

## 2. Scope Resolution

- [ ] 2.1 Implement suppression attachment: pragma immediately before a `CodeUnit` suppresses that unit
- [ ] 2.2 Implement module-level suppression: pragma at file top (before any `CodeUnit`) suppresses the entire `Module`
- [ ] 2.3 Ensure suppression attachment is performed during CST traversal before metrics are computed

## 3. Engine Integration

- [ ] 3.1 Thread suppression list into the check pipeline
- [ ] 3.2 Skip rule evaluation for a `CodeUnit` if it carries a matching active suppression
- [ ] 3.3 Skip module-level rule evaluation if the `Module` carries a matching active suppression

## 4. CLI Subcommand

- [ ] 4.1 Implement `pretender suppressions list` — scans source files, reports each suppression: file, function/module, rules, reason, expiry
- [ ] 4.2 Include expired suppressions in the list with a clear `[EXPIRED]` annotation

## 5. Plugin Manifest Extension

- [ ] 5.1 Add `[suppressions]` key parsing in language plugin `plugin.toml` loader
- [ ] 5.2 Allow plugins to specify `comment_prefix` and `comment_style` (inline vs block)

## 6. Tests

- [ ] 6.1 Unit test: pragma with valid reason and rules parses correctly
- [ ] 6.2 Unit test: pragma with empty reason emits validation error
- [ ] 6.3 Unit test: pragma with `allow[*]` emits validation error
- [ ] 6.4 Unit test: expired pragma is ignored and violation resurfaces
- [ ] 6.5 Integration test: suppressed function does not appear in check results
- [ ] 6.6 Integration test: `suppressions list` enumerates all pragmas including expired ones
