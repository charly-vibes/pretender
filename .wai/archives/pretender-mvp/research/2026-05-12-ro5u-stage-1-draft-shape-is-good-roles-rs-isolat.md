---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-82e-role-detection-path-glob-matching, pipeline-step:review]
---

RO5U: Stage 1 draft shape is good: roles.rs isolates detection from CLI and config.rs. Stage 2 correctness found pragma parsing was too strict for whitespace/block comments, so parse_role_pragma now accepts  and trims block-comment suffixes; added coverage. Stage 3 clarity: Role, RoleDetector, and EffectiveThresholds names communicate intent; specificity-based glob conflict resolution is explicit. Stage 4 edge cases: tests cover pragma precedence, specific glob precedence over broad defaults, heuristics fallback, app fallback, and threshold override application. Stage 5 excellence: cargo fmt and cargo test -p pretender --quiet green.
