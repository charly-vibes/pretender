---
tags: [pipeline-run:tdd-ro5u-2026-11-24-dogfood-pretender-gate-in-ci, pipeline-step:plan]
---

# Plan: dogfood pretender's structural gate in this repo's CI

## Context
The repo already dogfoods pretender locally: `pretender.toml` (mode=tiered) plus
`pretender-baseline.json` (adopted 2026-11-19, 236 known findings) and a lefthook
pre-commit hook `pretender check --mode gate`. But the gate is local-only:
`.github/workflows/ci.yml` runs `just ci` = fmt-check + type-check + lint + test,
never the structural gate. Anyone without lefthook installed (or using
`--no-verify`) bypasses it entirely.

Also: `pretender.toml` sets `mode = "tiered"` with no `advisory_until` lease, so the
command AGENTS.md tells contributors to run (`pretender check . --staged`) errors out
("mode=tiered requires a current advisory lease"). The lefthook hook only works
because it overrides the mode with `--mode gate`. Config and hook disagree.

## Desired end state
1. `just dogfood` runs the structural gate on the repo's own source.
2. CI (`.github/workflows/ci.yml`) runs the same gate on every PR and push to main,
   using the binary built from this checkout.
3. `pretender.toml` is the single source of truth for the mode (gate), so bare
   `pretender check .` behaves identically to the hook and to CI.
4. Test fixtures (deliberate counterexamples) are excluded from the gate scope so
   the signal is production code only.
5. The baseline is refreshed and stable across repeated scans (no flapping).

## Phases
- Phase 1 (RED): a repo-level test asserting the gate is wired — justfile has a
  `dogfood` recipe, ci.yml runs it, pretender.toml is in gate mode and excludes
  fixtures. Test fails first.
- Phase 2 (GREEN): add `just dogfood`, add the CI step (build release binary, run
  the gate), set `mode = "gate"`, exclude `**/fixtures/**` from the scan.
- Phase 3 (TIDY): refresh the baseline (`pretender check . --adopt`), re-run the
  gate three times to confirm a stable known-count and 0 new findings; verify
  `just ci` and the new test pass; run `ah check` (spec-test correspondence).

## Out of scope
- Changing any rule outcome, rule default threshold, or the tool's behaviour
  (spec says rule defaults need an openspec proposal).
- Refactoring the code to satisfy tighter thresholds — that is the separate epic
  ("thresholds until the code is very easy to understand").
- Fixing the known measurement gaps (Rust `params` capture: pretender-6tt;
  void_mutator flapping: pretender-gv5). Those are inputs to the epic, not this
  change.

## Risks
- Gate flapping (pretender-gv5) could make CI fail spuriously on a changed tree;
  mitigated by adopting a baseline that covers both detector states and by
  confirming stability with repeated scans before merging.
- Switching the config to gate mode turns previously-advisory findings into hard
  failures for anyone running the bare command. Intended: the baseline keeps known
  findings green, only new ones fail.

