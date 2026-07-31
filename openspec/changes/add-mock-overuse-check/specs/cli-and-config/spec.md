## ADDED Requirements

### Requirement: Mock-Overuse Detection

The system SHALL detect mock, stub, fake, and spy references in test files during `pretender check` and SHALL flag files whose mock-reference count exceeds the configured `[thresholds.test] mock_count_max` value.

#### Scenario: Mock count exceeds threshold in gate mode
- **WHEN** a test file contains 5 mock *usage* references (mock construction or expectation calls) and `mock_count_max` is 3 in gate mode
- **THEN** the system emits a mock-overuse finding for that file and the exit code indicates a violation

#### Scenario: Mock count within threshold
- **WHEN** a test file contains 5 mock usage references and `mock_count_max` is 5
- **THEN** the system emits no mock-overuse finding for that file

#### Scenario: Mock count equal to threshold does not trigger
- **WHEN** a test file contains exactly 3 mock usage references and `mock_count_max` is 3
- **THEN** the system emits no mock-overuse finding

#### Scenario: Infrastructure references excluded
- **WHEN** a test file imports `mockall` and uses `#[automock]` but never constructs a mock object
- **THEN** the mock count is 0 and no finding is emitted

#### Scenario: Disabled by default
- **WHEN** `mock_count_max` is not configured (defaults to 0)
- **THEN** the system performs no mock-reference detection

#### Scenario: Non-test role files are never evaluated
- **WHEN** a file has role `app` or `library` and contains mock references
- **THEN** the system emits no mock-overuse finding for that file

#### Scenario: Generated and vendor files excluded
- **WHEN** a file has role `generated` or `vendor`
- **THEN** the system performs no mock-reference detection for that file

### Requirement: Mock Finding Output

Mock-overuse findings SHALL be emitted in `human`, `json`, and `sarif` output formats and SHALL include the file path, mock-reference count, configured limit, and locations of each detected reference.

#### Scenario: Human format includes mock details
- **WHEN** `pretender check --format human` runs on a file with mock-overuse findings
- **THEN** the output includes the file path, mock count, limit, and per-reference locations

#### Scenario: SARIF emits dedicated rule ID
- **WHEN** `pretender check --format sarif` runs on a file with mock-overuse findings
- **THEN** the SARIF output includes results with rule id `pretender/mock-overuse`

## ADDED Requirements

### Requirement: Configurable Mock Threshold

The system SHALL support a `mock_count_max` field under `[thresholds.test]` in `pretender.toml`, defaulting to 0 (disabled).

#### Scenario: Explicit threshold configured
- **WHEN** `[thresholds.test] mock_count_max = 5` is set in `pretender.toml`
- **THEN** files with role `test` are evaluated against threshold 5

#### Scenario: Zero threshold disables check
- **WHEN** `mock_count_max = 0`
- **THEN** the system performs no mock-reference detection

#### Scenario: Custom mock patterns via config
- **WHEN** `[patterns.mock] extra = ["my_lib::Mock"]` is set in `pretender.toml`
- **THEN** the system also detects `my_lib::Mock` as a mock reference in addition to built-in patterns