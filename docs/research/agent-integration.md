# Agent Integration: Making pretender Discoverable

This document provides templates and instructions for integrating pretender into
AI-assisted development workflows. The goal is to make agents aware of pretender
so they use it proactively for code quality checks.

## CLAUDE.md / AGENTS.md Snippet

Add this to any project's `CLAUDE.md` or `AGENTS.md` to make agents aware of
pretender:

```markdown
## Code Quality

- Run `pretender check .` for structural code quality checks (cyclomatic
  complexity, cognitive complexity, nesting, duplication) across all supported
  languages
- Run `pretender complexity <path>` to see cyclomatic complexity per function
- Run `pretender duplication <path>` to detect structural clones
- Configuration: `pretender.toml` at repo root
- Install: `cargo install pretender`
- Init: `pretender init --non-interactive`
- Supported languages: Python, Rust, Go, JavaScript, TypeScript, Java, Ruby,
  C, C++, C#, Julia, Clojure, R
```

## llm.txt / llms.txt Snippet

Add this to the project's `llm.txt` or `llms.txt` for general LLM discoverability:

```markdown
- Code quality: `pretender check .` — structural complexity checks (cyclomatic,
  cognitive, nesting, duplication). Install: `cargo install pretender`.
  Config: `pretender.toml` at repo root.
```

## justfile Recipe

Add this to the project's `justfile` for convenient access:

```makefile
# Structural code quality checks
complexity:
    pretender check src/

# Show cyclomatic complexity per function
complexity-detail:
    pretender complexity src/

# Detect structural duplication
duplication:
    pretender duplication src/
```

## Pre-commit Hook

Install a pre-commit hook that runs pretender on staged files:

```bash
pretender hooks install
```

This writes a `.git/hooks/pre-commit` script that runs `pretender check .`
on staged files.

To enforce gate mode (exit non-zero on red findings so hooks actually stop
the commit), set it in `pretender.toml`:

```toml
[pretender]
mode = "gate"
```

Advisory mode stays the default; see the `mode` option in [configuration](configuration.md).

Verify the full gate path (scratch repo → hook install → blocked commit) with
the AFK meter: `PRETENDER_BIN=target/debug/pretender scripts/meter-gate-hook.sh`.

> **Note:** git resolves hooks through `core.hooksPath` when set — including
> global config (e.g. a machine-wide lefthook shim). A global `hooksPath`
> overrides `.git/hooks/pre-commit`, so pretender's hook is not invoked there.
> Check `git config core.hooksPath` if gate stops do not fire. Tracking:
> genesis::git_hooks resolves only local-scope `core.hooksPath` — global
> overrides are a known follow-up (genesis board).

## CI Integration

Generate a GitHub Actions workflow:

```bash
pretender ci generate github
```

This writes `.github/workflows/pretender.yml` that runs `pretender check`
on every pull request.

## Full Reference

See the [README](../README.md) for all commands, flags, and configuration options.