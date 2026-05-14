## ADDED Requirements

### Requirement: ClonePair Type

The system SHALL define a `ClonePair` type with fields: `pair_id: u64`, `similarity: u8` (0–100), `size_nodes: u32`, and `locations: [Location; 2]`. A `Location` SHALL contain `file: PathBuf` and `span: Span`. `pair_id` SHALL be computed as a hash of the two location strings sorted lexicographically, ensuring stability across reruns and file reordering.

#### Scenario: Stable pair_id for symmetric pair
- **WHEN** a clone pair (A, B) is detected where A and B are in different files
- **THEN** the `pair_id` for (A, B) equals the `pair_id` for (B, A)

#### Scenario: pair_id changes when span changes
- **WHEN** one location's span changes (edit within a clone site)
- **THEN** the `pair_id` of that pair changes

### Requirement: Duplication Ratio Formula

The system SHALL compute `duplication_ratio` as `duplicated_nodes / total_nodes` where `duplicated_nodes` is the count of AST nodes participating in any detected clone pair (counted once per node regardless of how many pairs it appears in), and `total_nodes` is the total AST node count for the scanned scope.

#### Scenario: No clones yields ratio of zero
- **WHEN** no clone pairs are detected
- **THEN** `duplication_ratio = 0.0`

#### Scenario: Ratio is bounded
- **WHEN** `duplication_ratio` is computed for any input
- **THEN** `0.0 <= duplication_ratio <= 1.0`
