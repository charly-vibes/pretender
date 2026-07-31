# Change: Add coupling and cohesion metrics

## Why

Pretender currently measures per-function complexity but has no module-level or cross-module metrics. High coupling (Ce, Ca, CBO) makes modules fragile and hard to extract; low cohesion (LCOM) indicates classes with multiple responsibilities; cycles in the dependency graph prevent independent deployment. These are the most actionable structural metrics after complexity, and teams need a deterministic CI gate to prevent architectural drift.

## What Changes

- Add a **module-level** check that computes per-module coupling metrics from the import graph and reports violations against configurable thresholds.
- Introduce new coupling metric thresholds: `ce_max` (efferent coupling), `ca_max` (afferent coupling), `cbo_max` (coupling between objects), `lcom_hs_max` (lack of cohesion, Henderson-Sellers), and a `cycle_detection` boolean flag.
- All thresholds go under `[thresholds]` root (not role-specific initially — coupling is a module-level concern, not role-specific).
- Coupling metrics are computed from the `imports` field of the universal code model, which currently emits an empty list — this change requires populating it.
- **Module boundary definition**: A module is a single source file (one `Module` in the universal model). For OOP languages with packages, the module path is the file's package-qualified path. Ce/Ca/CBO are computed per-file module. LCOM-HS is computed per-file for class-based languages only (Java, C++, C#, Python classes, Rust impl blocks); for non-OOP modules (Go packages, Rust modules without classes), LCOM-HS is not computed (value = 0, no finding emitted).
- **Cross-file resolution**: Single-file analysis only. Import edges between files are resolved within the same check run (parsed files form the graph). Inheritance depth is resolved within the same file only — cross-file parent classes are marked as unknown depth.
- Introduce `ModuleReport` as a new top-level section in `CheckReport`, separate from `FileReport` (coupling metrics span files, not individual units).
- Cycle detection walks the directed import graph and reports each cycle with its participant modules.
- Output in `human`, `json`, and `sarif` formats; `gate` mode fails on any cycle or any threshold violation.
- Excludes files with role `generated` or `vendor` from the dependency graph and all coupling analysis.

## Impact

- Affected specs: `cli-and-config` (MODIFIED Thresholds Schema, Check Report; ADDED Coupling Metric Thresholds, Cycle Detection), `universal-code-model` (MODIFIED Import Resolution — imports must be populated from language adapters).
- Affected code: `pretender/src/config.rs` (new coupling threshold fields), `pretender/src/main.rs` (new coupling evaluation path, `ModuleReport` in `CheckReport`, extended `decide_exit_code`), new `pretender/src/coupling.rs` (import-graph building, Ce/Ca/CBO/LCOM-HS computation, cycle detection), each language adapter (populate `Module.imports`).
- New dependency: none — graph algorithms are pure Rust (DFS for cycles, simple counting for coupling).
- SARIF rule IDs: `pretender/efferent-coupling`, `pretender/afferent-coupling`, `pretender/coupling-between-objects`, `pretender/lcom-hs`, `pretender/import-cycle`.
- Non-breaking: all new thresholds default to `0` (disabled) or `false` (cycle detection off); existing checks unchanged.

## Dependencies

- **Foundational** — requires populating `Module.imports` across all language adapters (task 3), which is a prerequisite for all other module-level checks.