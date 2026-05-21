# Feedback Loop Design

## Status

Design draft for `pretender-5rk`.

## Problem

Pretender currently computes structural metrics and emits per-run findings, but those signals are ephemeral. Teams cannot answer:

- which violations keep recurring
- which areas of the codebase are chronic weak spots
- whether a project should tighten or relax defaults
- how agent/tooling should incorporate known structural risks into future work

The feedback loop needs two layers:

1. **Violation emission** from `pretender check`
2. **Violation persistence + aggregation** across runs

## Current Architecture Constraints

Relevant current behavior:

- `pretender/src/metrics.rs` computes `cyclomatic`, `cognitive`, `function_lines`, `params`, `nesting_max`, and `abc`.
- `pretender/src/roles.rs` derives `EffectiveThresholds` per role.
- `pretender/src/main.rs` already builds `UnitReport` / `FileReport` with threshold violations for file and unit metrics.
- `pretender check --format json` persists the last run to `.pretender/last-check.json`.
- `tiered` uses bands for advisory coloring; `gate` uses threshold violations for failure.

That means the missing piece is not metric calculation. It is durable modeling of findings over time.

## Goals

- Preserve structural findings beyond a single CLI run.
- Detect recurring violations with project-local evidence.
- Feed recurring patterns back into:
  - OpenSpec proposal constraints
  - Pretender project defaults / profiles
  - Agent prompts / system context
- Keep the first implementation simple, local, and explainable.

## Non-Goals

- Cloud sync or central multi-repo telemetry in V1.
- Automatic threshold mutation without human review.
- Fully generic analytics for every future metric plugin.
- Direct autonomous edits to OpenSpec specs or prompts.

## Recommended Violation Taxonomy

Use a two-level taxonomy.

### 1. Canonical rule key

Persist violations under stable rule identifiers derived from current threshold semantics:

- `file_lines_max`
- `cyclomatic_max`
- `cognitive_max`
- `function_lines_max`
- `params_max`
- `nesting_max`
- `abc_max`
- `min_assertions`
- `exported_params_max`
- `exported_cyclomatic_max`
- `exported_lines_max`
- later: `duplication_pct_max`, `coverage_line_min`, `coverage_branch_min`, `mutation_min`

Why this level:

- aligns with existing config keys
- preserves direct traceability to `pretender.toml`
- makes profile adjustment straightforward

### 2. Classification facets

Each stored event should also carry normalized facets:

- `metric_family`: `complexity | size | interface | test-quality | duplication | coverage | mutation`
- `scope`: `file | unit | exported-unit | project`
- `role`: `app | test | library | script | generated | vendor`
- `severity`: `yellow | red | gate-fail`
- `source`: `built-in-metric | external-plugin`

Why this second level:

- allows richer aggregation without inventing a second rule system
- supports prompt injection like “tests often violate assertion minimums”
- future-proofs external plugins

## Violation Event Model

Persist each finding as an append-only event.

```json
{
  "schema_version": 1,
  "timestamp": "2026-05-20T14:32:11Z",
  "repo_id": "pretender",
  "run_id": "uuid-or-timestamp-hash",
  "mode": "tiered",
  "path": "src/parser.py",
  "unit_name": "parse_module",
  "role": "app",
  "rule_key": "cognitive_max",
  "metric_family": "complexity",
  "scope": "unit",
  "severity": "red",
  "actual": 22,
  "limit": 15,
  "delta": 7,
  "fingerprint": "stable-location-or-symbol-hash"
}
```

### Fingerprint

Use a stable fingerprint to distinguish “same chronic hotspot” from “many unrelated violations”.

Recommended V1 fingerprint:

- `path + unit_name + rule_key` for unit findings
- `path + rule_key` for file findings

Recommended V2 refinement:

- add symbol span normalization or AST-derived symbol identity to survive line movement

## Emission Path

Add a post-analysis step in `pretender check`:

1. analyze files into `CheckReport`
2. derive threshold violations exactly as today
3. convert each violation into a `ViolationEvent`
4. persist events if enabled
5. compute recurrence summaries
6. optionally render recurrence hints in human/markdown/html output

This should be a separate transformation layer, not embedded into metric calculation.

### Why separate it

- metrics remain pure
- report rendering remains format-focused
- persistence can evolve independently
- external plugins can emit into the same event model later

## Persistence Model

### Recommended V1 storage

Store project-local data under:

- `.pretender/history/events.jsonl`
- `.pretender/history/summaries.json`

`events.jsonl`:
- append-only audit log
- easy to inspect
- resilient to partial writes

`summaries.json`:
- cached aggregates for fast reads
- rebuildable from the event log

### Why project-local first

- matches current local cache conventions
- keeps privacy/simple ownership clear
- works offline and in CI
- makes review/debugging easy

### Retention

Recommended defaults:

- keep last 90 days of events
- cap total events file size to a modest bound (for example 10–25 MB)
- prune on write, not as a background service

## Aggregation Dimensions

Aggregate across three primary dimensions.

### 1. Per project

This is the baseline and should be mandatory.

Questions answered:
- what rule is most often violated here?
- what roles are most problematic?
- is the repo getting healthier or noisier?

### 2. Per code area

Aggregate by path prefix, for example:

- `src/compiler/`
- `tests/integration/`
- `pkg/api/`

This is the most useful operational slice because recurring structural issues are usually local to an area.

Recommended heuristic:
- first meaningful directory prefix
- optionally configurable later

### 3. Per agent/model/run source

Do **not** make this core V1 behavior, but reserve it in the event schema.

Useful future dimensions:
- `actor_type`: `human | agent | ci`
- `actor_id`: model or workflow identifier

This is valuable for studying whether a specific model tends to create certain defects, but it should remain optional because:
- the CLI often runs without reliable actor identity
- it introduces privacy/attribution questions
- it is not required to unlock project-level feedback

## What Counts as “Recurring”

Use two recurrence thresholds: hotspot recurrence and pattern recurrence.

### Hotspot recurrence

A hotspot is recurring when the same fingerprint appears:

- at least **3 times** across
- at least **2 distinct days**
- within the retention window

Why:
- avoids flagging one noisy local loop
- identifies chronic code that survives edits/reviews

### Pattern recurrence

A pattern is recurring when the same `rule_key` appears:

- at least **5 times**
- across at least **3 distinct files**
- within the same role or code area

Why:
- distinguishes systemic style/architecture problems from one bad function

## Recommended Severity Interpretation

Persist both band and threshold semantics.

- `yellow`: advisory band breach only
- `red`: configured threshold violation
- `gate-fail`: a red violation observed in `gate` mode during a failing run

This matters because:
- yellow findings are useful for trend detection before they become hard failures
- red findings map to current config semantics
- gate-fail is especially useful for CI and policy integration

## Integration Recommendations

### 1. OpenSpec constraints

Do not auto-edit specs.

Instead, generate a machine-readable and human-readable recurrence summary that proposal tooling can ingest.

Example constraint suggestions:

- “Tests in `tests/integration/` repeatedly violate `min_assertions`.”
- “Library exports frequently exceed `exported_params_max`; prefer narrower public APIs.”
- “Parser code repeatedly exceeds `cognitive_max`; require decomposition in future changes.”

Recommended flow:
- `pretender feedback summarize` or equivalent future command emits structured constraints
- OpenSpec proposal tooling offers “suggested project constraints” to the author
- human accepts/rejects

### 2. Pretender profiles / defaults

Do not auto-rewrite `pretender.toml` in V1.

Instead, offer recommendations:

- tighten a threshold only when a team is consistently clean
- relax a threshold only when persistent violations are intentional and accepted
- prefer area-specific or role-specific suggestions over global changes

Good examples:
- “Consider `[thresholds.test].min_assertions = 2` once the suite is consistently compliant.”
- “Consider explicit `[roles.script]` or path-based overrides for `scripts/**` if script files repeatedly exceed app defaults.”

### 3. Agent prompts

This is the highest-value short-term integration.

Expose a concise “known weak spots” summary that can be injected into prompts, for example:

- chronic high cognitive complexity in `src/parser/`
- repeated exported API bloat in `pkg/api/`
- weak assertion density in integration tests

Rules for prompt injection:
- summarize only top 3–5 recurring patterns
- prefer actionable wording
- include confidence/occurrence counts
- never dump raw event logs into prompts

## Suggested Data Products

A future implementation should produce three outputs.

### 1. Raw events

For debugging and audit.

### 2. Aggregated summary

For humans and agent context.

Example:

```json
{
  "top_patterns": [
    {
      "rule_key": "cognitive_max",
      "role": "app",
      "area": "src/parser",
      "count": 8,
      "distinct_files": 4,
      "distinct_days": 5
    }
  ],
  "top_hotspots": [
    {
      "fingerprint": "src/parser.py::parse_module::cognitive_max",
      "count": 4,
      "distinct_days": 3
    }
  ]
}
```

### 3. Constraint suggestions

Small, human-reviewable recommendations derived from summaries.

## Recommended Implementation Sequence

### Phase 1: tracer bullet

Implement one end-to-end signal only:

- `cognitive_max`

Deliver:
- event emission from `pretender check`
- local JSONL persistence
- one summary view of recurring hotspots

Why first:
- easy to observe
- high signal for agent-generated code
- exercises all feedback-loop stages

### Phase 2: expand built-in rules

Add:
- `cyclomatic_max`
- `nesting_max`
- `params_max`
- `function_lines_max`
- `file_lines_max`
- `abc_max`
- `min_assertions`

### Phase 3: project/system integrations

Add:
- prompt-summary export
- profile recommendation output
- OpenSpec constraint suggestion output

### Phase 4: plugin and project-level signals

Add:
- duplication
- coverage
- mutation
- external metric wrappers

## Key Decisions

- **Decision:** Use existing threshold keys as the canonical rule taxonomy.
  - **Why:** direct config traceability and low migration cost.
- **Decision:** Persist append-only local events in project storage.
  - **Why:** simple, inspectable, offline-first.
- **Decision:** Separate hotspot recurrence from pattern recurrence.
  - **Why:** teams need both “this function is chronic” and “this area has a systemic problem”.
- **Decision:** Recommend; do not auto-edit OpenSpec specs, prompts, or config.
  - **Why:** preserves human review and avoids unsafe policy drift.
- **Decision:** Start with one tracer-bullet signal (`cognitive_max`).
  - **Why:** smallest path to prove the loop end-to-end.

## Risks and Trade-offs

### False persistence after refactors

A path/name-based fingerprint may overcount renamed or moved code.

Mitigation:
- keep V1 simple
- evolve to symbol/AST-derived identities later

### Noisy advisory findings

Persisting yellow-band signals can create too much volume.

Mitigation:
- persist yellow events only when explicitly enabled, or summarize them separately
- default recurrence calculations to red/gate-fail first

### Policy ossification

Automatically tightening thresholds based on history could encode temporary local pain as permanent policy.

Mitigation:
- recommendations only
- require human acceptance for config/spec changes

### Storage growth

Long-lived repos can accumulate large event histories.

Mitigation:
- retention window + size cap + prune-on-write

## Proposed Follow-up Implementation Ticket

Create a follow-up issue for:

**Title:** Implement tracer-bullet feedback loop for recurring `cognitive_max` violations

Scope:
- emit `ViolationEvent`s for `cognitive_max`
- persist to `.pretender/history/events.jsonl`
- compute recurring hotspot summary
- expose summary in a simple JSON or markdown command/output
- add tests for recurrence thresholds and retention pruning

## Recommendation

Approve this design and implement the tracer bullet around `cognitive_max` only before generalizing to the full rule set.
