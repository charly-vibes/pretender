# Pre-Refactor Snapshot — structural-clarity epic (pretender-5zo)

Captured 2026-11-24, before the first refactor slice landed. Purpose: a fixed
reference point for burn-down metrics and later guidance. Do not edit; write a
new dated snapshot alongside this one.

- **Tag:** `pre-refactor` → `2d963252aece6b3dad224c07fa713b96f0106e15`
  (merge of PR #61, issue-review remediation)
- **Gate verdict at capture:** `gate: 164 known (baselined), 0 new`
- **Baseline ledger:** `pretender-baseline.json`, schema 1,
  `finding_ids` length = **164**, adopted_at 2026-11-24T21:03:55Z

## Source size (the burn-down primary metric)

| File | Lines |
|---|---|
| pretender/src/main.rs | 3977 |
| pretender/src/engine.rs | 1749 |
| pretender/src/mutation.rs | 990 |
| pretender/src/config.rs | 982 |
| pretender/src/test_report.rs | 869 |
| pretender/src/history.rs | 812 |
| pretender/src/roles.rs | 563 |
| pretender/src/mutability_metrics.rs | 453 |
| pretender/src/model.rs | 451 |
| pretender/src/inheritance_depth.rs | 368 |
| pretender/src/coupling.rs | 366 |
| pretender/src/external_plugin.rs | 343 |
| pretender/src/mock_detector.rs | 338 |
| pretender/src/doctor.rs | 338 |
| pretender/src/primitive_obsession.rs | 312 |
| pretender/src/metrics.rs | 294 |
| pretender/src/cluster_detector.rs | 273 |
| pretender/src/error_metrics.rs | 259 |
| pretender/src/duplication.rs | 248 |
| pretender/src/typescript.rs | 213 |
| pretender/src/data_model.rs | 213 |
| **src total (all .rs)** | **16062** |
| pretender/tests/cli_test.rs | 3900 |
| pretender/tests/genesis_compile_test.rs | 278 |
| pretender/tests/repo_dogfood_test.rs | 88 |
| **tests total** | **4377** |

## Baseline composition (grandfathered debt by rule, top offenders)

Derived from `pretender-baseline.json` finding IDs at capture time — see the
committed baseline for the authoritative list. Rule keys observed in IDs:
`function_lines_max`, `excessive_fields`, `file_lines`, `unwraps`,
`abc_max`, `mock_count`, `lazy_cluster`, `cyclomatic`, `nesting`, and others.

## Gate config at capture (pretender.toml)

- `mode = "gate"`, `languages = ["auto"]`
- `exclude = ["vendor/**", "node_modules/**", "**/*_generated.*", "**/fixtures/**"]`
- Roles: test / library / script / generated / vendor (paths as committed)
- Thresholds: defaults in effect (no `[thresholds.*]` overrides committed yet;
  e.g. app `params_max = 4` — silently unenforced for Rust per pretender-6tt)

## Known truth-blockers at capture

- `pretender-6tt` — Rust extractor never populates `params` (params_max=4
  unenforced on our own code). Openspec proposal required before the fix.
- `pretender-gv5` — void_mutator detection flaps 55↔49 known on an unchanged
  tree; CI flake risk now that the gate runs in CI.

## Validation state at capture

Prior session reported `just ci` green (gate 164/0, three identical
consecutive scans), `ah check` clean, bd PASS-0 clean, no dep cycles.
Re-verified at capture: gate run `164 known, 0 new`.
