## 1. Fingerprint and Baseline Data Model

- [ ] 1.1 Define `Fingerprint` struct: `(file, unit_name, unit_start_line, rule)`
- [ ] 1.2 Implement `bucket(value, threshold)` — coarse bucketing so minor fluctuations don't invalidate
- [ ] 1.3 Define `BaselineEntry` struct: `{ fingerprint: String, rule, file, unit, unit_start_line, value, threshold, bucket }`
- [ ] 1.4 Define `BaselineFile` struct: `{ version: u32, created_at: DateTime, findings: Vec<BaselineEntry> }`

## 2. Baseline File I/O

- [ ] 2.1 Implement `BaselineStore::load(path) -> Result<BaselineFile>`
- [ ] 2.2 Implement `BaselineStore::save(path, baseline) -> Result<()>`
- [ ] 2.3 Honor `[baseline] path` config key (default: `pretender.baseline.json`)

## 3. CLI Subcommands

- [ ] 3.1 Implement `pretender baseline create` — runs full check, snapshots findings, writes `pretender.baseline.json`
- [ ] 3.2 Implement `pretender baseline update` — re-runs check, replaces baseline file
- [ ] 3.3 Implement `pretender baseline show` — loads baseline file, prints table of grandfathered findings

## 4. Check Integration

- [ ] 4.1 Add `--baseline` flag to `pretender check`
- [ ] 4.2 On `--baseline`: load baseline file; for each finding, look up fingerprint
- [ ] 4.3 Pass (exit 0) if finding is in baseline AND value ≤ baselined value
- [ ] 4.4 Fail (exit non-zero) if finding is NOT in baseline OR value > baselined value (regression)
- [ ] 4.5 If `auto_update_improved = true` and value < baselined value: silently write tighter baseline entry

## 5. Configuration

- [ ] 5.1 Add `[baseline]` table parsing: `path`, `auto_update_improved`
- [ ] 5.2 Wire config values to `BaselineStore` construction

## 6. Tests

- [ ] 6.1 Unit test: bucket function stability around threshold boundaries
- [ ] 6.2 Integration test: create baseline, introduce regression, verify exit non-zero
- [ ] 6.3 Integration test: improve a finding, verify auto_update tightens baseline
- [ ] 6.4 Test `baseline show` output format
