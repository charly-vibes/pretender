# Why Agents Don't Use pretender Frequently

**Research Date:** 2026-07-17
**Author:** charly vibes
**Status:** Published (corrected after Rule of 5 review)

## Executive Summary

pretender is a structural code quality CLI for the charly ecosystem, but
agents invoke it far less than sibling tools (`dont`, `wai`, `testaruda`,
`espectacular`). This research investigates why, drawing on data from pi
session logs, project configurations, and toolchain integration analysis.

**Key finding:** pretender is not integrated into the `wai way` project
setup workflow, has no discoverability path in pi prompt templates, and
agents struggle with its CLI. The fixes are well-understood and
high-leverage.

## Methodology

- Analyzed 566 pi session JSONL files across all projects
- Counted project config directories (`.pretender/`, `.dont/`, etc.)
- Counted justfile and CI references
- Examined `wai way` source code for integration gaps
- Verified findings against pi prompt template system

## Session Data

### Tool Invocation Frequency (all sessions)

| Tool | Sessions Referencing It | Notes |
|------|-----------------------|-------|
| openspec | 419 | Spec-driven development |
| bd (beads) | 417 | Issue tracking |
| wai | 405 | Workflow manager |
| dont | 200 | Claims verification |
| espectacular | 119 | Spec-test correspondence |
| testaruda | 70 | Test selection |
| **pretender** | **55** | **Code quality — lowest in toolchain** |

### Corrected Invocation Count

The 55 pretender invocations break down as:

- **41** — from pretender's own development sessions (running `pretender check`
  as part of testing the tool itself)
- **14** — from non-pretender projects (the real agent adoption number)

**The headline number is 14, not 55.** All comparisons below use 14.

### How wai way Works

The `wai way` command runs a series of `check_*()` functions, each
diagnosing one aspect of a project (e.g., `check_editorconfig()`,
`check_typos()`, `check_test_coverage()`). Each function returns a
pass/info result. When a check fails, it suggests a fix. The checks are
registered in a `vec![...]` and run sequentially. Adding a new check
requires: (1) writing the function, (2) adding it to the vec, (3) adding
tests. This is the mechanism by which `check_pretender()` was added
(see tickets pretender-65j, pretender-7kj).

### Project Adoption

| Metric | pretender | dont | testaruda | wai |
|--------|-----------|------|-----------|-----|
| Config dirs | 4 projects | 9 | 10 | 28 |
| Justfile mentions | 5 | 3 | 4 | 15 |
| Agent invocations | 14 (real) | 200 | 70 | 405 |

### Indirect Usage

| Path | Count | Projects |
|------|-------|----------|
| Direct `pretender` invocations | 14 | Various |
| justfile wrappers | 5 projects | pretender, testaruda, smithy, Testimonial.jl, Tray.jl |
| CI integration | 3 projects | cositos, smithy, Testimonial.jl |
| Pre-commit hooks | 3 projects | smithy, Testimonial.jl, Tray.jl |
| CLAUDE.md/AGENTS.md mention | 1 project | cositos |

**No hidden indirect usage** was found in pi session logs — agents do not
invoke justfile wrappers that call pretender.

## Root Causes

### 1. Not part of the "wai way" project setup

The `wai way` command is the standard project initialization workflow — it
checks 16 things (task runner, git hooks, editorconfig, typos, vale, CI/CD,
test coverage, beads, openspec, etc.). **pretender was completely absent**
from this system. Every other tool in the charly ecosystem has a path to
discovery through wai; pretender didn't.

**Fix applied:** Added `check_pretender()` to wai way (pretender-65j).
Also added `wai way code-quality` topic guide (pretender-7kj).

### 2. Zero discoverability in pi prompt templates

pretender is mentioned in **0 of 60+ pi skill prompt templates**. The only
way an agent discovers pretender is through project-level context files
(CLAUDE.md, AGENTS.md, justfile), and only **1 project** (cositos)
mentions it there.

**Fix applied:** Created `docs/agent-integration.md` with copyable
CLAUDE.md/AGENTS.md snippets (pretender-pgp).

### 3. CLI friction

Session data shows agents repeatedly struggling with:

- `pretender check --threshold guidance` — wrong flag, should be `--mode`
  (3 instances)
- `pretender check` with no path — missing required argument (4 instances)
- `pretender init` issues — interactive mode blocks on prompts (2 instances)
- Various `pretender not found` checks (agents probing availability)

**Fix applied:** `pretender check`, `complexity`, and `duplication` now
default to the current directory when no path is given (pretender-1rz).

### 4. Minimal adoption across projects

Only 4 projects have `.pretender/` config, 5 have justfile references,
and 1 mentions pretender in CLAUDE.md. Compare to `dont` (9 config dirs)
and `wai` (28).

### 5. No feedback loop to justify itself

The beads issue `pretender-5rk` ("Explore feedback loop: track recurring
structural violations") was designed but never implemented. There's no
mechanism for pretender to surface its value over time.

**Fix pending:** Design doc for feedback loop (pretender-42d) and
implementation (pretender-3nh).

### 6. No pi custom tool (ecosystem-wide)

None of the charly tools have pi custom tools or extensions. This is not
pretender-specific, but it means pretender can only be invoked via `bash`,
which is less discoverable than a registered tool call.

## Assumptions

This research assumes **increased pretender adoption is the goal**. If the
user is intentionally deferring adoption (e.g., waiting for the feedback
loop feature before promoting pretender), some recommendations may be
premature. The author is aware of this assumption.

## Prioritization of Recommendations

| # | Fix | Effort | Impact | Status |
|---|-----|--------|--------|--------|
| 1 | Add `check_pretender()` to wai way | ~50 lines Rust | High | ✅ Done |
| 2 | Fix CLI to default to repo root | CLI change + tests | High | ✅ Done |
| 3 | Create CLAUDE.md/AGENTS.md template | One doc file | Medium | ✅ Done |
| 4 | Add wai way code-quality guide topic | ~100 lines Rust | Medium | ✅ Done |
| 5 | Implement feedback loop | Design + impl | High | 🔜 Pending (pretender-42d, 3nh) |
| 6 | Create pi custom tool | Extension dev | Low | 📋 Not started |
| 7 | Measure success metrics | Ongoing | — | ✅ Done |

## Success Metrics

| Metric | Baseline | 2-Month Target |
|--------|----------|----------------|
| Non-pretender invocations | 14 | 30 |
| Projects with `pretender.toml` | 5 | 8 |
| Projects with justfile refs | 5 | 10 |
| Projects with CLAUDE.md mention | 1 | 5 |
| `wai way` shows pretender pass | 0 | All charly projects |

See `docs/research/success-metrics.md` for full details.

## References

- `docs/research/indirect-usage-audit.md` — indirect usage measurement
- `docs/research/success-metrics.md` — adoption targets and measurement
- `docs/agent-integration.md` — CLAUDE.md/AGENTS.md template snippets
- `pretender/src/main.rs` — CLI source (fixed for default path behavior)
- `wai/src/commands/way/mod.rs` — wai way check system (pretender added)