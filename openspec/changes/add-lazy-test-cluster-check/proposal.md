# Change: Add lazy-test-cluster check

## Why

Test suites with repeated identical assertion patterns (same SUT, same assertion structure, different literals) disguise regression holes as coverage — N nearly-identical tests are not N independent checks. Detecting these clusters deterministically lets teams collapse them into parameterized tests, reducing maintenance surface and clarifying which variations actually cover different behaviors.

## What Changes

- Add a **post-parse** cluster analysis that groups test code units within a file by (a) same called function / SUT, (b) same assertion kind, (c) structurally identical body shape differing only in literal values. Structural comparison includes assertion count, assertion types, call-site count, and branching structure — not just assertion count alone.
- Introduce `lazy_cluster_min: u32` under `[thresholds.test]` (default `0` = disabled); clusters of size ≥ `lazy_cluster_min` are flagged as findings.
- Operates on the existing `CodeUnit` tree (not a separate parser) — assertion patterns and call sites are already captured in the AST model.
- **Limitation**: Clustering is single-file only. Identical assertion patterns across multiple test files are not detected.
- Excludes files with role `generated` or `vendor`.
- Cluster findings attach to `FileReport` as `lazy_cluster_findings`; gatable in `gate` mode.
- Purely static — requires the test file only.

## Impact

- Affected specs: `cli-and-config` (ADDED lazy-cluster threshold and report field).
- Affected code: `pretender/src/config.rs` (new `lazy_cluster_min`), `pretender/src/main.rs` (cluster-analysis invocation, new finding, exit-code extension), new `pretender/src/cluster_detector.rs` (AST comparison + cluster grouping logic).
- No new dependencies — operates on the existing universal code model.
- Non-breaking: default `0` disables the check.

## Dependencies

- None (independent).