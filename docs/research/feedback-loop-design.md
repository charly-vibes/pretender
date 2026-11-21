# Feedback Loop: Recuring Violation Tracking

**Status:** Approved
**Date:** 2026-07-18
**Design Doc:** v1

## Overview

The feedback loop persists structural violations across check runs so agents
can see trends and recurring patterns. This gives pretender a reason to keep
being used — it's not just a pass/fail gate, but a memory of code quality
over time.

## Current State

The feedback loop is **already implemented** in V1 scope (Phase 1). The
history module at `pretender/src/history.rs` provides:

- **`ViolationEvent`** — schema v1 with timestamp, run_id, path, unit_name,
  role, rule_key, metric_family, severity, actual, limit, delta, fingerprint
- **`EventStore`** — append-and-prune persistence to `.pretender/history/events.jsonl`
  with 90-day retention and 10MB cap
- **`compute_summary()`** — generates hotspots (same fingerprint, 3+ occurrences,
  2+ days) and patterns (same rule_key + role + area, 5+ occurrences, 3+ files)
- **Integration** — `emit_history_events()` is called at the end of `CheckArgs::run()`;
  recurrence hints are printed in human output
- **Tests** — 9 unit tests covering retention, thresholds, date math, path area

## Design Decisions

### 1. Violation Taxonomy

**Decision:** Use pretender's existing threshold categories as the violation rule_key.

Each metric maps to a rule_key:
- `cognitive_max` → cognitive complexity
- `cyclomatic_max` → cyclomatic complexity (Phase 2)
- `params_max` → parameter count (Phase 2)
- `nesting_max` → nesting depth (Phase 2)
- `function_lines_max` → function length (Phase 2)
- `abc_max` → ABC score (Phase 2)
- `duplication_pct_max` → duplication percentage (Phase 2)
- `min_assertions` → test assertion count (Phase 2)

**Rationale:** The threshold system already defines these categories in
`config.rs`. Reusing them avoids creating a parallel taxonomy. The
`metric_family` field provides a higher-level grouping ("complexity",
"size", "duplication").

### 2. Aggregation Strategy

**Decision:** Two levels of aggregation:

1. **Hotspots** — per-fingerprint aggregation. A fingerprint is
   `{path}::{unit_name}::{rule_key}`. This tracks "this exact function keeps
   violating this rule." Threshold: 3+ occurrences across 2+ days.

2. **Patterns** — per (rule_key, role, area) aggregation. The "area" is the
   first two directory components of the path (e.g., `src/parser` for
   `src/parser/module.rs`). This tracks "cognitive violations keep happening
   in the parser module." Threshold: 5+ occurrences across 3+ files.

**Rationale:** Hotspots catch specific functions that need refactoring.
Patterns catch systemic issues across a code area. Different granularities
for different interventions.

### 3. Auto-Generation

**Decision:** Deferred to separate tickets.

- **openspec constraints** — Not in scope. Would require a mechanism to
  generate openspec capability constraints from recurring patterns. See
  follow-up ticket (untracked).
- **Agent prompt injection** — Not in scope. Would require injecting
  "known weak spots" into system prompts. See follow-up ticket (untracked).

**Rationale:** The V1 feedback loop establishes the data pipeline
(persist → aggregate → surface). What consumes the data is a separate
concern that can be iterated on independently.

### 4. Recurrence Thresholds

**Decision:**

| Threshold | Value | Rationale |
|-----------|-------|-----------|
| `HOTSPOT_MIN_COUNT` | 3 | 3+ violations of the same function = pattern |
| `HOTSPOT_MIN_DAYS` | 2 | Must span at least 2 days (not a single session) |
| `PATTERN_MIN_COUNT` | 5 | 5+ violations across the code area |
| `PATTERN_MIN_FILES` | 3 | Must span at least 3 files (not one file) |
| `RETENTION_DAYS` | 90 | 3-month window for trend analysis |
| `MAX_BYTES` | 10 MB | Safety cap for the event log |

**Rationale:** These thresholds were chosen to minimize false positives
while still catching meaningful patterns. A single bad commit triggers
multiple violations but they're all on the same day (HOTSPOT_MIN_DAYS=2
filters them out). A function that keeps getting flagged across 3 different
sessions is a real problem.

## Phase Plan

### Phase 1 (current, ✅ Done)
- [x] ViolationEvent schema v1
- [x] EventStore with append, prune, retention
- [x] Hotspot and pattern detection
- [x] Cognitive complexity only (cognitive_max)
- [x] Integration in human output
- [x] Unit tests

### Phase 2 (this ticket: pretender-3nh)
- [ ] Extend `cognitive_max_events()` to all metrics (cyclomatic, params,
      nesting, function_lines, abc, duplication, min_assertions)
- [ ] Add `--format json` output for `HistorySummary` (currently only human)
- [ ] Add `--format json` history section to `pretender report` output
- [ ] Add yellow-band and gate-fail severity levels (currently only red)
- [ ] Add integration tests for the full pipeline

### Phase 3 (follow-up tickets)
- [ ] openspec constraint generation from recurring patterns
- [ ] Agent prompt injection for known weak spots
- [ ] Trend visualization (line charts per metric over time)

## References

- `pretender/src/history.rs` — full implementation
- `pretender/src/main.rs` — `emit_history_events()` integration
- `pretender.exe.config.rs` — threshold definitions