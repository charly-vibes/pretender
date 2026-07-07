---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-82e-role-detection-path-glob-matching, pipeline-step:plan]
---

pretender-82e role detection plan: Desired end state is a role detection layer that classifies files as app/library/test/script/generated/vendor using priority explicit pragma in leading comments, then configured path glob patterns, then built-in heuristics, and exposes effective threshold selection for a detected role before metric evaluation. Out of scope: check CLI integration, actual metric violation evaluation, generated/vendor skip semantics, and glob-based file discovery. Phases: RED add role detection and threshold-selection tests; GREEN implement Role enum, RoleDetector with globset matching/pragma scan/heuristics, and EffectiveThresholds from Config thresholds; RO5U review priority and edge cases; VERIFY cargo test -p pretender.
