## MODIFIED Requirements

### Requirement: Per-Clone-Pair Duplication Output

The system SHALL emit one finding per clone pair from `pretender duplication` instead of an aggregate percentage. Each finding SHALL include `pair_id`, `similarity` (integer 0–100), `size_nodes`, and two `locations` (each with `file` and `span`). **BREAKING**: replaces the previous single aggregate percentage output.

#### Scenario: Single clone pair found within a file
- **WHEN** two structurally identical subtrees of ≥10 nodes exist in the same file
- **THEN** `pretender duplication` emits exactly one finding with both locations pointing into that file

#### Scenario: Cross-file clone pair found
- **WHEN** two structurally identical subtrees exist in different files
- **THEN** `pretender duplication` emits one finding with `locations[0]` in the first file and `locations[1]` in the second file

#### Scenario: No clones found
- **WHEN** no subtree pairs meet the minimum size and similarity thresholds
- **THEN** `pretender duplication` emits an empty `clone_pairs` array and `duplication_ratio = 0.0`

### Requirement: Cross-File Scanning Default

The system SHALL enable cross-file clone detection by default. **BREAKING**: the `--cross-file` opt-in flag from V0/V1 is removed. A `--no-cross-file` flag SHALL be added to restrict scanning to within-file pairs only.

#### Scenario: Default run includes cross-file pairs
- **WHEN** `pretender duplication` is run with no flags
- **THEN** clone pairs spanning different files are included in output

#### Scenario: Opting out of cross-file scanning
- **WHEN** `pretender duplication --no-cross-file` is run
- **THEN** only within-file clone pairs are reported

### Requirement: Duplication Filtering Flags

The system SHALL support `--min-clone-size <nodes>` (default 10) and `--min-similarity <0-100>` (default 90) flags on `pretender duplication`. Only pairs meeting both thresholds SHALL be emitted.

#### Scenario: Minimum clone size filters small pairs
- **WHEN** `--min-clone-size 20` is passed and a clone pair has `size_nodes = 15`
- **THEN** that pair is excluded from output

#### Scenario: Minimum similarity filters near-clones
- **WHEN** `--min-similarity 100` is passed
- **THEN** only exact structural clones (similarity = 100) are reported

### Requirement: Duplication Ratio Threshold Gating

The system SHALL compute `duplication_ratio = duplicated_nodes / total_nodes` across all discovered pairs. The existing `duplication_pct_max` config key SHALL gate on `duplication_ratio * 100`. In `gate` mode, if `duplication_ratio * 100 > duplication_pct_max`, the system SHALL exit non-zero.

#### Scenario: Gate mode fails on high ratio
- **WHEN** `duplication_pct_max = 5` and `duplication_ratio = 0.08`
- **THEN** `pretender check` in `gate` mode exits non-zero

#### Scenario: Gate mode passes on acceptable ratio
- **WHEN** `duplication_pct_max = 5` and `duplication_ratio = 0.03`
- **THEN** `pretender check` in `gate` mode exits zero

### Requirement: SARIF Clone Pair Output

Each clone pair SHALL be emitted as one SARIF `result`. The primary location SHALL be `locations[0]`. The paired site SHALL be `relatedLocations[0]`. The `message.text` SHALL include `pair_id`, `similarity`, and `size_nodes`.

#### Scenario: SARIF result for a single clone pair
- **WHEN** one clone pair is detected and output format is `sarif`
- **THEN** the SARIF report contains exactly one `result` with one entry in `locations` and one entry in `relatedLocations`

#### Scenario: SARIF is valid SARIF 2.1.0
- **WHEN** `pretender duplication --format sarif` runs on any input
- **THEN** the output validates against the OASIS SARIF 2.1.0 schema
