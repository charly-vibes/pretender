## 1. SARIF Fix Infrastructure

- [ ] 1.1 Define `FixSuggestion` type: rule id, description, artifact changes (path, region, replacement text)
- [ ] 1.2 Add `fix_suggestions(finding: &Finding) -> Vec<FixSuggestion>` dispatch function
- [ ] 1.3 Wire `fix_suggestions` into SARIF emitter — populate `result.fixes` when non-empty
- [ ] 1.4 Confirm fixes are absent from non-SARIF output formats

## 2. Per-Rule Fix Strategies

- [ ] 2.1 `min_assertions` — insert TODO comment at line 1 of function body span
- [ ] 2.2 `params_max` — insert grouping comment listing parameter names
- [ ] 2.3 `function_lines_max` — identify largest nested block by span.lines(); insert extraction comment above it
- [ ] 2.4 `cyclomatic_max` / `cognitive_max` — identify highest-weight branch from metric data; insert comment pointing to it
- [ ] 2.5 `duplication` — insert cross-reference comment on both clone sides (location A references B, B references A)

## 3. Rule ID Tagging

- [ ] 3.1 Each `FixSuggestion` includes `rule_id` field matching the finding's `ruleId` in SARIF
- [ ] 3.2 Confirm SARIF output links fix to parent result via matching `ruleId`

## 4. Tests

- [ ] 4.1 Snapshot test: SARIF output for `min_assertions` finding includes `fixes` block
- [ ] 4.2 Snapshot test: SARIF output for `params_max` finding includes parameter names in fix text
- [ ] 4.3 Snapshot test: `duplication` finding emits two results each with a cross-reference fix
- [ ] 4.4 Test: rule with no mechanical fix (e.g., `nesting_max`) produces no `fixes` key in SARIF
- [ ] 4.5 Test: non-SARIF format (`human`, `json`) produces no fix suggestions in output
