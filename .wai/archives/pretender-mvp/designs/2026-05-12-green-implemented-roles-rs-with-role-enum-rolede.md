---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-82e-role-detection-path-glob-matching, pipeline-step:green]
---

GREEN: Implemented roles.rs with Role enum, RoleDetector using globset-compiled [roles] patterns, explicit leading comment pragma parsing ( / ), glob matching before heuristics, and app fallback. Glob conflicts choose the most specific matched pattern by non-wildcard character count so configured narrower patterns can override broad defaults. Added heuristic detection for vendor/generated/test/library/script paths. Added EffectiveThresholds::for_role to apply test, library, and script overrides from Config before metric evaluation. Verified roles::tests and cargo test -p pretender --quiet green.
