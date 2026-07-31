## ADDED Requirements

### Requirement: Lazy Test Cluster Threshold

The system SHALL accept a `lazy_cluster_min: u32` field under `[thresholds.test]` in `pretender.toml`, defaulting to `0` (disabled). When enabled, test code units with identical assertion structure and SUT call differing only in literal values SHALL be grouped into clusters, and clusters of size ≥ `lazy_cluster_min` SHALL be flagged.

#### Scenario: Lazy test cluster exceeds threshold
- **WHEN** a test file has 5 test bodies with identical assertion structure, branching, and SUT call differing only in literal values, and `lazy_cluster_min = 3`
- **THEN** the system emits a lazy-cluster finding for that file

#### Scenario: Same assertion count but different structure not clustered
- **WHEN** two test bodies have the same assertion count but different branching or different assertion types
- **THEN** the system emits no finding (false-positive rejection)

#### Scenario: Cluster size below threshold
- **WHEN** a test file has 2 identical test bodies and `lazy_cluster_min = 3`
- **THEN** the system emits no finding

#### Scenario: Zero threshold disables the check
- **WHEN** `lazy_cluster_min = 0`
- **THEN** no cluster analysis is performed

#### Scenario: Generated and vendor files excluded
- **WHEN** a file has role `generated` or `vendor`
- **THEN** the system performs no cluster analysis for that file

### Requirement: Lazy Cluster Finding Output

The system SHALL emit lazy-cluster findings in `human`, `json`, and `sarif` output formats with the finding name `lazy_cluster`, including cluster size, SUT name, and first/last line numbers.

#### Scenario: SARIF rule ID
- **WHEN** `pretender check --format sarif` runs on a file with a lazy-cluster finding
- **THEN** the SARIF output includes results with rule id `pretender/lazy-cluster`