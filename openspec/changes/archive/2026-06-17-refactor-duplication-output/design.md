## Context

`pretender duplication` previously emitted a single aggregate percentage, making it impossible to act on specific clone pairs. This change switches to per-pair findings, which is a breaking output change affecting JSON consumers, SARIF shape, and the default cross-file behavior.

## Goals / Non-Goals

- Goals: per-pair findings with stable IDs; cross-file on by default; SARIF with relatedLocations; `duplication_ratio` formula
- Non-Goals: fuzzy/semantic clone detection (stays structural AST hashing); IDE quick-fix integration; deduplication of transitive clone clusters

## Decisions

- Decision: `--cross-file` becomes the default; `--no-cross-file` opts out
  - Rationale: Cross-file clones are the most actionable case; within-file-only caused false negatives. The V0 opt-in was a performance hedge that the incremental cache makes unnecessary.
  - Alternative: Keep opt-in, require explicit `cross_file = true` in config — rejected because it perpetuates the false-negative default.

- Decision: `pair_id` is a hash of the sorted (location_a, location_b) pair strings
  - Rationale: Stable across reruns; deterministic; survives file renames if content is identical.
  - Alternative: Sequential integer — rejected because it shifts on any ordering change.

- Decision: `duplication_ratio = duplicated_nodes / total_nodes` (not line-based)
  - Rationale: Consistent with the AST-node unit used for clone detection; resistant to reformatting noise.
  - Alternative: Line-based percentage — rejected because lines are cosmetic, not structural.

- Decision: `duplication_pct_max` key is kept unchanged; its value is now compared against `duplication_ratio * 100`
  - Rationale: Minimizes migration cost for existing config files; numeric values are compatible.

## Risks / Trade-offs

- Large codebases with many clones may produce voluminous JSON → mitigated by `--min-clone-size` and `--min-similarity` defaults
- SARIF consumers must update field mapping (breaking) → migration note in CHANGELOG

## Migration Plan

1. Update JSON consumers: replace `duplication_pct` field with `duplication_ratio` + `clone_pairs` array
2. SARIF consumers: each finding now has `relatedLocations`; update any tooling that assumed single-location duplication results
3. Config: `duplication_pct_max` value is unchanged; no toml edits required
4. Rollback: revert to previous binary; `duplication_pct_max` still parses correctly

## Open Questions

- Should `duplication_ratio` be per-file or project-wide for threshold gating? (Current decision: project-wide, consistent with old behavior)
