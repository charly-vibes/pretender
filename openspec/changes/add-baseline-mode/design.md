## Context

When `pretender` is introduced to an existing codebase in `gate` mode, all pre-existing violations
must be fixed before the gate becomes usable. That is unrealistic for large legacy codebases.
Baseline mode solves this by capturing the current violation state so only new or worsening findings
block the build. The most consequential design decision is the fingerprint: it must be stable enough
to survive refactors but sensitive enough to catch genuine regressions.

## Goals / Non-Goals

- **Goals:**
  - Allow `gate` mode to be enabled on day 1 of a legacy codebase without requiring pre-existing fixes
  - Block new violations and regressions relative to the baseline
  - Ratchet: baselined values can only improve; any regression triggers failure
  - Stable fingerprints across metric-value changes inside the same unit (no false invalidations)

- **Non-Goals:**
  - Automatic mass-cleanup of grandfathered findings (out of scope)
  - Per-developer baselines (single shared `pretender.baseline.json` committed to the repo)
  - Cross-language aggregated baselines in V1

## Decisions

### Decision 1: Fingerprint = (file, unit_name, unit_start_line, rule); bucket is stored separately

**Why:** A fingerprint that includes the exact metric value would invalidate on any edit to the
function, even improvements. A fingerprint that uses only `(file, unit_name, rule)` cannot disambiguate repeated or nested units with the same name in one file. Including the unit start line makes same-name units distinct while keeping the identity independent of the current metric value. Bucketing the value (e.g., 5-unit buckets relative to the threshold) is stored separately so minor churn does not fail the baseline, but genuine regressions (crossing a bucket boundary upward) do.

**Bucketing scheme:** `bucket = floor(value / max(1, threshold / 5))`. This produces ~5 "coarse
buckets" per threshold unit. A function with cyclomatic complexity 23 (threshold 10) is in bucket 2;
editing it to 25 stays in bucket 2 (pass); editing to 30 enters bucket 3 (fail).

**Alternatives considered:**
- Exact value: too fragile — any refactor within the function invalidates.
- Content hash of function body: unstable across whitespace and comment changes.
- Exact line-only anchor: unstable across insertions above the function. The selected design uses start line only as a same-name disambiguator; moving a unit intentionally creates a new baseline identity.

### Decision 2: Baseline file is committed to version control

**Why:** The baseline is a team agreement, not a personal setting. It should be reviewed in PRs,
audited in git history, and shared across CI and developer machines.

**Format:**
```json
{
  "version": 1,
  "created_at": "2026-05-14T00:00:00Z",
  "findings": [
    {
      "fingerprint": "sha256:...",
      "rule": "cyclomatic",
      "file": "src/parser.rs",
      "unit": "parse_expr",
      "unit_start_line": 42,
      "value": 23,
      "threshold": 10
    }
  ]
}
```

### Decision 3: Ratchet behavior — auto-tighten on improvement

**Why:** Without a ratchet, the baseline never shrinks and there is no incentive to clean up
grandfathered findings. With `auto_update_improved = true` (default), when a finding's value
drops below the baselined value, the baseline entry is silently updated to the new (lower) value.
This means it is impossible to re-introduce the previously grandfathered value without failing.

### Decision 4: SHA-256 fingerprint hash

The fingerprint string stored in the JSON is `sha256(file_path + "\x00" + unit_name + "\x00" + unit_start_line + "\x00" + rule)`.
This allows the JSON to be compact and the fingerprint to be stable across metric-value changes
that don't affect the file path, unit name, unit start line, or rule.

## Risks / Trade-offs

- **Risk: Unit rename breaks fingerprint** → Mitigation: `baseline update` must be run after
  intentional renames. This is acceptable; a rename is a deliberate change.
- **Risk: Baseline file grows unbounded** → Mitigation: `baseline update` prunes entries for
  units that no longer exist.
- **Risk: Race condition on `auto_update_improved` write** → Mitigation: update is written
  atomically via `.tmp` → `rename`; worst case is a missed tightening (safe).

## Migration Plan

No breaking changes. The `--baseline` flag to `check` is opt-in. The baseline file is created
explicitly by `pretender baseline create`. Projects that do not use baseline mode are unaffected.

## Open Questions

- Should `pretender baseline show` support `--format json` output? (Proposed: yes, for tooling.)
- Should there be a `pretender baseline diff` subcommand showing delta since last update? (Deferred to V2.)
