## 1. Data Model

- [ ] 1.1 Define `ClonePair` struct: `pair_id: u64`, `similarity: u8` (0–100), `size_nodes: u32`, `locations: [Location; 2]`
- [ ] 1.2 Define `Location` struct: `file: PathBuf`, `span: Span`
- [ ] 1.3 Remove the existing aggregate `duplication_pct` output field from the metric result type
- [ ] 1.4 Add `duplication_ratio: f64` field to module-level summary (duplicated_nodes / total_nodes)

## 2. Detection Engine

- [ ] 2.1 Enable cross-file subtree hash collection by default (remove the `--cross-file` opt-in gate)
- [ ] 2.2 Add `--no-cross-file` flag that restricts collection to within-file pairs only
- [ ] 2.3 Add `--min-clone-size <nodes>` flag (default 10); wire to subtree hash filter
- [ ] 2.4 Add `--min-similarity <0-100>` flag (default 90); wire to pair scoring filter
- [ ] 2.5 Assign stable `pair_id` values (hash of sorted location pair) for deterministic output

## 3. Threshold Gating

- [ ] 3.1 Compute `duplication_ratio = duplicated_nodes / total_nodes` across all discovered pairs
- [ ] 3.2 Gate `duplication_pct_max` on `duplication_ratio` (rename threshold key in docs; value semantics unchanged)
- [ ] 3.3 Ensure `gate` mode fails when `duplication_ratio > duplication_pct_max / 100`

## 4. Output Formats

- [ ] 4.1 Update `human` output: print one line per clone pair with similarity %, size, and both locations
- [ ] 4.2 Update `json` output: emit `clone_pairs` array with `ClonePair` entries and `duplication_ratio` summary
- [ ] 4.3 Update `sarif` output: each `ClonePair` is one `result`; `locations[0]` = primary site; `relatedLocations[0]` = paired site
- [ ] 4.4 Update `markdown` and `junit` renderers to consume `clone_pairs`

## 5. Config & CLI

- [ ] 5.1 Update `pretender.toml` schema docs: note `duplication_pct_max` now gates `duplication_ratio`
- [ ] 5.2 Document `--min-clone-size`, `--min-similarity`, `--no-cross-file` in CLI help text
- [ ] 5.3 Update `pretender explain duplication` to print the `duplication_ratio` formula

## 6. Tests

- [ ] 6.1 Unit test: `duplication_ratio` formula with known node counts
- [ ] 6.2 Integration test: within-file clone pair detected and emitted with correct `pair_id`, `similarity`, `size_nodes`
- [ ] 6.3 Integration test: cross-file clone pair detected by default (no flag required)
- [ ] 6.4 Integration test: `--no-cross-file` restricts to within-file pairs only
- [ ] 6.5 Integration test: `--min-similarity 100` filters near-clones
- [ ] 6.6 Snapshot test: SARIF output for a single clone pair has exactly one `result` with `relatedLocations`
