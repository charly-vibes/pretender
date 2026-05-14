## 1. Argument Parsing

- [ ] 1.1 Extend `pretender explain` to accept `<file>::<function>` and `<finding-id>` forms
- [ ] 1.2 Detect argument form: bare metric name (existing), `::` separator (finding by location), or finding id prefix (`PRT-`)
- [ ] 1.3 Route to existing metric explanation path when argument is a bare metric name

## 2. Cache Lookup

- [ ] 2.1 Read cached metric results from last `pretender check` run (requires `add-incremental-cache`)
- [ ] 2.2 Look up cache entry by `<file>::<function>` key
- [ ] 2.3 Look up cache entry by finding id (SARIF result id)
- [ ] 2.4 Emit actionable error when no cache entry found: "No cached results for '<arg>'. Run 'pretender check' first."

## 3. Output Sections

- [ ] 3.1 Section 1 (existing): rule name, definition, citation, threshold
- [ ] 3.2 Section 2 "This finding": file path+span, function name, actual score, zone label (green/yellow/red)
- [ ] 3.3 Section 3 "Top contributors": ranked list of line number + node description + contribution value (top 5)
- [ ] 3.4 Section 4 "What helps": 2-3 mechanical suggestions + suppression pragma template
- [ ] 3.5 Render all sections in `human` format with clear visual separation

## 4. Suppression Pragma Template

- [ ] 4.1 Emit the correct suppression pragma comment syntax for the detected language
- [ ] 4.2 Include the rule id in the pragma (e.g., `// pretender:ignore PRT-CYCLO`)

## 5. Tests

- [ ] 5.1 Integration test: `explain src/router.rs::handle_request` with warm cache prints all 4 sections
- [ ] 5.2 Integration test: `explain PRT-CYCLO-abc123` resolves to same output as location form
- [ ] 5.3 Test: missing cache entry returns non-zero exit with correct error message
- [ ] 5.4 Test: bare metric name still works (backward compatibility)
- [ ] 5.5 Snapshot test: "Top contributors" lists up to 5 items sorted by contribution value descending
