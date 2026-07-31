## 1. Configuration

- [ ] 1.1 Add coupling threshold fields to `[thresholds]` in `config.rs`:
  - `ce_max: u32` (default 0)
  - `ca_max: u32` (default 0)
  - `cbo_max: u32` (default 0)
  - `lcom_hs_max: u32` (default 0) — LCOM-HS percentage (0-100)
  - `cycle_detection: bool` (default false)
- [ ] 1.2 Validate: all threshold fields are non-negative u32; `lcom_hs_max` ≤ 100; `cycle_detection` is boolean.
- [ ] 1.3 Unit tests for config parse + defaults + validation.

## 2. Import-graph building

- [ ] 2.1 Create `pretender/src/coupling.rs` with data structures:
  - `DepGraph` — adjacency list of `(module_path, Vec<dep>)` edges
  - `ModuleDeps` — per-module: `Ce` (distinct external deps), `Ca` (distinct dependents), `CBO` (Ce + Ca), `LCOM_HS` (Lack of Cohesion of Methods, Henderson-Sellers formula)
- [ ] 2.2 **Module boundary definition**: A module = one source file. The module path is the file path relative to the project root. Ce/Ca/CBO computed per-file. For OOP languages (Java, C++, C#, Python classes, Rust impl blocks), LCOM-HS is computed per-class; for non-OOP modules, LCOM is not computed (value = 0, no finding emitted).
- [ ] 2.3 Build the graph from `Module.imports` across all parsed files in a check run. Each import resolves to a module path (relative to project root or known library prefix). Exclude files with role `generated` or `vendor` from the graph.
- [ ] 2.3 Compute Ce per module: count of distinct external modules it imports.
- [ ] 2.4 Compute Ca per module: count of distinct modules that import it.
- [ ] 2.5 Compute CBO per module: `Ce + Ca` (distinct external couplings).
- [ ] 2.6 Compute LCOM-HS: `LCOM-HS = (M - sum(methods accessing each field) / F) / (M - 1)` where M = method count, F = field count. Requires per-method field-access analysis — start with a heuristic: if M ≤ 1 or F = 0, LCOM-HS = 0; otherwise compute from AST.
- [ ] 2.7 Implement cycle detection: DFS with back-edge detection on the directed import graph. Report each cycle as a list of participant module paths.
- [ ] 2.8 Unit tests for graph building, Ce/Ca/CBO computation, LCOM-HS (with fixture AST data), and cycle detection (acyclic, single cycle, multi-cycle, self-loop edge cases).

## 3. Populating imports in language adapters

- [ ] 3.1 Audit each language adapter's `.scm` query to determine if `@import` captures are already present. Add `@import` captures where missing, mapping to the `Import` struct in the universal model.
- [ ] 3.2 For each language, define the import-capture query pattern:
  - Rust: `use` / `extern crate`
  - Python: `import` / `from ... import`
  - JavaScript/TS: `import`, `require`
  - Java: `import`
  - Go: `import`
  - Ruby: `require`, `require_relative`
  - C/C++: `#include`
  - Julia: `using`, `import`
  - R: `library`, `require`
  - C#: `using`
  - Clojure: `:require`, `import`
- [ ] 3.3 Update each language adapter to populate `Module.imports`. Test each adapter against fixture files with known import statements.
- [ ] 3.4 Unit tests per language: fixture file's imports are correctly parsed and exposed in the `Module.imports` list.

## 4. Evaluation and reporting

- [ ] 4.1 Create `ModuleReport` struct:
  ```rust
  struct ModuleReport {
      path: String,
      coupling_violations: Vec<CouplingViolation>,
  }
  struct CouplingViolation {
      metric: String,  // "ce", "ca", "cbo", "lcom_hs"
      actual: f64,
      limit: f64,
  }
  ```
- [ ] 4.2 Add `modules: Vec<ModuleReport>` to `CheckReport` (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`).
- [ ] 4.3 Add `cycles: Vec<Vec<String>>` to `CheckReport` for cycle detection results.
- [ ] 4.4 Run coupling evaluation after all files are parsed: build the dep graph, compute metrics, compare against thresholds, assemble module reports.
- [ ] 4.5 Extend `decide_exit_code` to treat any coupling violation or cycle as a gate-mode violation.
- [ ] 4.6 Human format: render module reports with metric name, actual, limit per module; render cycles as `Cycle: A → B → C → A`.
- [ ] 4.7 JSON format: include `modules` and `cycles` fields.
- [ ] 4.8 SARIF format: emit coupling violations as results with rule ids `pretender/efferent-coupling`, `pretender/afferent-coupling`, `pretender/coupling-between-objects`, `pretender/lcom-hs`; emit cycles as `pretender/import-cycle`.
- [ ] 4.9 Persist module reports and cycles in the report cache.

## 5. Integration tests

- [ ] 5.1 Fixture project with 3 modules forming a cycle (A→B→C→A); verify cycle detection with `cycle_detection = true`.
- [ ] 5.2 Fixture project with high Ce (imports 10 externals, `ce_max = 5`); verify violation.
- [ ] 5.3 Fixture with high Ca (depended on by 8 modules, `ca_max = 5`); verify violation.
- [ ] 5.4 Fixture with high CBO; verify violation.
- [ ] 5.5 Fixture with LCOM-HS violation.
- [ ] 5.6 All thresholds at 0 (defaults): no violations; no cycles detected.
- [ ] 5.7 Human, JSON, and SARIF output formats each contain coupling findings.

## 6. Docs and validation

- [ ] 6.1 Document `[thresholds] ce_max`, `ca_max`, `cbo_max`, `lcom_hs_max`, `cycle_detection` in user docs.
- [ ] 6.2 Document what each metric means and recommended thresholds (Ce ≤ 9, CBO ≤ 9, LCOM-HS ≤ 30%, cycles = 0).
- [ ] 6.3 Run `openspec validate add-coupling-and-cohesion-metrics --strict` and resolve every issue.
- [ ] 6.4 Run `just ci` (fmt + type-check + clippy + test) green.