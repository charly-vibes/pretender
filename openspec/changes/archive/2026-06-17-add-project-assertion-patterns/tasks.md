## 1. Config Schema

- [ ] 1.1 Add `[assertions]` table parsing to config loader
- [ ] 1.2 Parse `patterns` as `Vec<String>` (glob strings); validate each entry is a non-empty string
- [ ] 1.3 Treat absent `[assertions]` table as equivalent to language-defaults-only (no change to existing behavior)
- [ ] 1.4 Treat `patterns = []` as explicit opt-out: disable language-default assertion patterns for this project

## 2. Assertion Detection Integration

- [ ] 2.1 In the assertion counter, merge project `patterns` with language-default `@assert.*` captures
- [ ] 2.2 Glob-match each `CallSite.callee` against the merged pattern set
- [ ] 2.3 Count a `CallSite` as an assertion if it matches any pattern in the merged set
- [ ] 2.4 Apply assertion counting only to files with role `test`

## 3. Tests

- [ ] 3.1 Unit test: project pattern `expect*` matches `expectEqual` and `expectThat`
- [ ] 3.2 Unit test: language-default patterns still active when `[assertions]` table is absent
- [ ] 3.3 Unit test: `patterns = []` disables language defaults; no calls counted as assertions
- [ ] 3.4 Unit test: non-test-role file is unaffected by `[assertions]` config
- [ ] 3.5 Integration test: test file using only `verifyInvariant` passes `min_assertions = 1` when `patterns = ["verify*"]`
