## 1. Configuration

- [ ] 1.1 Add `lazy_cluster_min: u32` to `[thresholds.test]` in `config.rs`; default `0`. Validate: non-negative u32.
- [ ] 1.2 Unit tests for config parse + defaults + validation.

## 2. Cluster detection

- [ ] 2.1 Create `pretender/src/cluster_detector.rs` exposing `find_lazy_clusters(units: &[CodeUnit]) -> Vec<LazyCluster>` where `LazyCluster` carries `sut_name: String`, `assertion_kind: String`, `count: u32`, and `locations: Vec<Span>`.
- [ ] 2.2 Implement cluster detection algorithm:
  - Group code units by `name` (SUT being tested, identified from call sites or function name).
  - Within each group, compare structural body shape: assertion count, assertion types (`@assert.*` captures), their positions relative to call sites, branching structure, and literal-value positions. Two units are a candidate pair only if they share ALL of: same assertion count, same assertion types, identical branching structure, and differ only in literal values.
  - False-positive rejection: if two units have the same assertion count but different branching structure or different assertion types, they are NOT a match.
  - Cluster size ≥ `lazy_cluster_min` = flagged.
- [ ] 2.3 Use the existing `CallSite` and `Branch` nodes from the universal model to identify SUT calls and assertion branching.
- [ ] 2.4 Exclude files with role `generated` or `vendor` from cluster analysis.
- [ ] 2.5 Unit tests: fixture test files with N identical test bodies, cluster detected correctly; different assertion structures do not cluster; N < threshold does not cluster; same assertion count + different branching structure does not cluster.
- [ ] 2.6 Document the single-file limitation in user docs: identical test patterns across multiple files are not detected.

## 3. Evaluation and reporting

- [ ] 3.1 Run cluster detection during `check` for every `test`-role file.
- [ ] 3.2 Add `lazy_cluster_findings: Vec<LazyClusterFinding>` to `FileReport` (serde skip when empty).
- [ ] 3.3 Extend `decide_exit_code` for gate mode.
- [ ] 3.4 Human, JSON, SARIF output formats.

## 4. Validation

- [ ] 4.1 Run `openspec validate add-lazy-test-cluster-check --strict`.
- [ ] 4.2 Run `just ci` green.