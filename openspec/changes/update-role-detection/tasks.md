## 1. Role Detection Logic

- [ ] 1.1 Remove the heuristic tier from `src/roles.rs` (or equivalent role resolution module)
- [ ] 1.2 Implement three-tier role resolution: pragma → glob → default `app`
- [ ] 1.3 Implement pragma scanner: read first 10 lines of each file; match `// pretender: role=<role>`
- [ ] 1.4 Ensure pragma scanner supports comment styles for non-C-family languages (e.g., `# pretender: role=<role>` for Python/Ruby/Shell)
- [ ] 1.5 Return a `RoleAssignment` value that records both the role and the assignment source (`Pragma`, `Glob(<pattern>)`, `Default`)

## 2. CLI Flag

- [ ] 2.1 Add `--explain-roles` flag to `pretender check`
- [ ] 2.2 When `--explain-roles` is set, print one line per file: `<file>  role=<role>  source=<pragma|glob:<pattern>|default>`
- [ ] 2.3 Output `--explain-roles` to stdout before any check results

## 3. Tests

- [ ] 3.1 Unit test: pragma in first line assigns correct role
- [ ] 3.2 Unit test: pragma on line 10 is detected; pragma on line 11 is ignored
- [ ] 3.3 Unit test: no pragma + matching glob assigns glob role
- [ ] 3.4 Unit test: no pragma + no matching glob assigns `app`
- [ ] 3.5 Integration test: `pretender check --explain-roles` output shows correct source for each file
- [ ] 3.6 Regression test: files that previously relied on heuristics now receive `app` role
