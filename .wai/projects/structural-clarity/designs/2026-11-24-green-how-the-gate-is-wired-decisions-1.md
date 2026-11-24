---
tags: [pipeline-run:tdd-ro5u-2026-11-24-dogfood-pretender-gate-in-ci, pipeline-step:green]
---

# GREEN: how the gate is wired

## Decisions
1. **One source of truth for the mode**: `pretender.toml` selects `mode = "gate"`.
   The lefthook hook's `--mode gate` flag becomes redundant (harmless). Rationale:
   the previous split (config=tiered, hook=gate) meant the command AGENTS.md
   mandates (`pretender check . --staged`) errored with "mode=tiered requires a
   current advisory lease" — a governance rule nobody could follow.

2. **Gate scope = production code**: `exclude` gains `**/fixtures/**`. The fixture
   trees are deliberate counterexamples used by the tool's own tests; gating them
   would mean every new smelly fixture fails CI. They are still analyzed by the
   tests themselves (which point at the fixture paths directly).

3. **Gate entry point = `just dogfood`** → `cargo run --quiet -p pretender -- check .`.
   Using cargo (not a PATH lookup) means the gate always runs the code in this
   checkout — no stale global binary, and in CI the debug binary is already built
   by `just test`, so the extra cost is one incremental link.

4. **CI wiring via `just ci`**: `ci: fmt-check type-check lint test dogfood`, so
   local `just ci`, `just preflight`, and the CI workflow all run the identical
   gate — one command, one definition. The workflow keeps a comment pointing at it.

5. **Baseline stays the ratchet**: known findings are baselined by
   `file::symbol::rule` (not line numbers), so refactors that move code do not
   silently un-baseline or re-baseline findings. New findings fail; known ones stay
   green. Re-adopt with `pretender check . --adopt` after an intentional refactor.

## Consequences
- CI now fails on new structural findings — the tool's own value proposition applied
  to itself.
- Contributors without lefthook get the same gate through `just ci`.
- `pretender.toml` mode change makes the bare command gating, which is stricter
  than the old tiered default: intentional, and the baseline absorbs the delta.

