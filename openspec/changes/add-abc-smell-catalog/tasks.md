## 1. Catalog Format and Loading

- [ ] 1.1 Define `SmellCatalog` struct: `version: u32`, `patterns: Vec<SmellPattern>`
- [ ] 1.2 Define `SmellPattern` struct: `capture: String`, `weight: f64`, `component: AbcComponent`, `rationale: String`
- [ ] 1.3 Define `AbcComponent` enum: `A | B | C`
- [ ] 1.4 Implement catalog loader: read from `~/.config/pretender/smell-weights/<language>.toml`; fall back to shipped embedded catalog if file absent
- [ ] 1.5 Implement pattern resolution: glob-match `CallSite.callee` against `capture` patterns; first match wins; no match → weight 1.0

## 2. Shipped Catalogs

- [ ] 2.1 Write Python catalog: `@call.eval` → weight 2.0 C, `@call.global_state` → weight 1.5 B, `@call.exec` → weight 2.0 C
- [ ] 2.2 Write JavaScript/TypeScript catalog: `@call.eval` → weight 2.0 C, `@call.global_state` → weight 1.5 B
- [ ] 2.3 Write Rust catalog: `@call.unsafe_fn` → weight 1.5 B, `@call.unwrap` → weight 1.2 C
- [ ] 2.4 Write Go catalog: `@call.recover` → weight 1.5 B
- [ ] 2.5 Bundle all shipped catalogs as embedded assets (e.g., `include_bytes!`) for single-binary distribution

## 3. ABC Engine Integration

- [ ] 3.1 Thread `SmellCatalog` into `count_calls_weighted` and `count_branches_weighted` metric functions
- [ ] 3.2 Ensure weight is applied per `CallSite.callee` lookup; unmatched calls use weight 1.0
- [ ] 3.3 Apply `component` field: A-component calls contribute to A, B to B, C to C in ABC vector

## 4. CLI

- [ ] 4.1 Implement `pretender explain abc --language <lang>` — prints active weight table (catalog + any local overrides) as a formatted table
- [ ] 4.2 Add `--format json` support to `pretender explain abc` for machine-readable weight export
- [ ] 4.3 Update `pretender explain abc` (no language flag) to list available languages

## 5. User Overrides

- [ ] 5.1 If `~/.config/pretender/smell-weights/<language>.toml` exists, it shadows the shipped catalog entirely
- [ ] 5.2 Document override procedure in `pretender explain abc` output

## 6. Tests

- [ ] 6.1 Unit test: pattern resolution for exact match, glob match, and no-match fallback
- [ ] 6.2 Unit test: `@call.eval` in Python produces weight 2.0 on the C component
- [ ] 6.3 Integration test: `pretender explain abc --language python` prints at least the built-in patterns
- [ ] 6.4 Integration test: user-supplied override file shadows built-in catalog
- [ ] 6.5 Snapshot test: `pretender explain abc --language rust --format json` output is stable
