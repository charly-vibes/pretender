## 1. Configuration

- [ ] 1.1 Add `mock_count_max: u32` to `[thresholds.test]` table in `config.rs`; default `0` (disabled). Add `[patterns.mock] extra: Vec<String>` for user-defined mock patterns (default empty). Validate that `mock_count_max` is a non-negative u32.
- [ ] 1.2 Extend `EffectiveThresholds::for_role` to include `mock_count_max` for the `test` role.
- [ ] 1.3 Unit tests for config parse + defaults + validation + custom patterns (extend `config.rs` tests).

## 2. Mock-reference detection

- [ ] 2.1 Create `pretender/src/mock_detector.rs` exposing `detect_mocks(path, source, language, role_detector, config) -> Vec<MockReference>`, where `MockReference` carries `name: String`, `span: Span`, `kind: MockKind` (`Library`, `Manual`, `Framework`, `Usage`), and `path: PathBuf`.
- [ ] 2.2 Define per-language mock-library registries. Minimum supported:
  - Rust: `mockall`, `mockito`, `mock`, `Mock*`, `#[mock]`, `#[automock]`
  - Python: `unittest.mock`, `mock`, `pytest-mock`, `MagicMock`, `Mock`, `patch`
  - JavaScript/TypeScript: `jest.mock`, `vi.mock`, `sinon`, `jest.fn()`, `jest.spyOn`, `vi.fn()`, `vi.spyOn`
  - Java: `Mockito`, `@Mock`, `EasyMock`, `Mockito.mock`, `PowerMock`
  - Ruby: `rspec-mocks`, `Mocha`, `minitest/mock`, `double`, `mock`, `stub`
  - Go: `testify/mock`, `gomock`, `mockgen`, `mock.Controller`
  - C/C++: GMock (`MOCK_METHOD`, `EXPECT_CALL`), FFF (`FAKE_VALUE_FUNC`)
- [ ] 2.3 Implement detection: scan source for import/use statements matching mock libraries, plus inline mock usage patterns (e.g., `Mock::new()`, `mockito::mock()`, `expect(...)`, `when(...)`). Distinguish **infrastructure** references (imports, `#[automock]`, `#[mock]` annotations) from **usage** references (actual mock construction/call expectations). Only usage references count toward the threshold.
- [ ] 2.4 Apply `R` and `Julia` mock detection if standard patterns exist (minimal set — `mockr`, `testthat::mock`, `FlexMock.jl`).
- [ ] 2.5 Exclude files with role `generated` or `vendor` from mock detection.
- [ ] 2.6 Unit tests per language with fixture source files containing mock references (both library and manual). Include test for infrastructure vs usage distinction.

## 3. Evaluation and reporting

- [ ] 3.1 Run `detect_mocks` during `check` for every file with role `test` (including sub-roles). Count `mock_count` per file.
- [ ] 3.2 Add `mock_findings: Vec<MockFinding>` field to `FileReport` (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`), where `MockFinding` carries `count: u32`, `limit: u32`, and `references: Vec<MockReference>`.
- [ ] 3.3 Extend `decide_exit_code` to treat a non-empty `mock_findings` as a violation in `gate` mode.
- [ ] 3.4 Human format: render `N / LIMIT mocks exceeded` per file with reference details.
- [ ] 3.5 JSON format: include `mock_findings` on the file report.
- [ ] 3.6 SARIF format: emit mock-overuse findings with rule id `pretender/mock-overuse`.
- [ ] 3.7 Persist mock findings in the report cache for `pretender report`.

## 4. CLI and integration

- [ ] 4.1 No new CLI flags — the check runs automatically when `mock_count_max > 0` in config.
- [ ] 4.2 Integration tests: fixture test file with 5 mocks, `mock_count_max = 3`, produces a finding; `mock_count_max = 5`, no finding; `mock_count_max = 0` (default), no finding; non-test-role file never evaluated.
- [ ] 4.3 Document `[thresholds.test] mock_count_max` in user docs with per-language mock library notes.

## 5. Validation

- [ ] 5.1 Run `openspec validate add-mock-overuse-check --strict` and resolve every issue.
- [ ] 5.2 Run `just ci` (fmt + type-check + clippy + test) green.