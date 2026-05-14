## Context

Pretender's check pipeline is synchronous: for each file, it parses a CST, constructs a `Module`,
and runs all metric functions. On a 200-file repo with warm CPU caches this takes ~4–8 s — well over
the 2 s pre-commit budget. The solution is a persistent, content-addressed cache keyed on inputs
rather than timestamps, so it is correct across branch switches and rebases.

This is a cross-cutting change: the cache layer sits between the CLI dispatcher and the engine,
touches serialization of internal model types, and introduces a new dependency on SHA-256 hashing.
A `design.md` is warranted to nail the key design decisions before coding.

## Goals / Non-Goals

- **Goals:**
  - Achieve <2 s wall time for `pretender check --staged` on repos ≤500 changed files (warm cache)
  - Zero false positives: stale cache entries MUST NOT be served after file or tool version changes
  - CI portability: `export`/`import` commands enable cache sharing across CI job steps
  - Minimal footprint: default cap of 1 GB, 30-day TTL; configurable

- **Non-Goals:**
  - Distributed or remote cache backends (out of scope for V1)
  - Caching `mutation` check results (mutation is non-deterministic by design)
  - Per-project cache isolation within a monorepo (single `<repo-id>` per git root)

## Decisions

### Decision 1: SHA-256 over mtime for invalidation key

**Why:** mtime changes on checkout/rebase even when content is identical; SHA-256 is content-stable
across all git operations. The performance cost of hashing is ~1 MB/s on modern hardware — negligible
for typical source files (≤50 KB each).

**Alternatives considered:**
- `(mtime, size)` — faster but fails on branch switch (same timestamp, different content).
- `git object hash` — requires a git context; breaks for untracked files and non-git environments.

### Decision 2: Flat `metrics/<sha256>` files, not a SQLite database

**Why:** Flat files are lock-free, trivially archivable via `tar`, and require no additional Rust
dependencies. Index is kept in `index.json` (path → current hash + last-used epoch).

**Alternatives considered:**
- SQLite via `rusqlite` — adds ~2 MB to binary, introduces write-lock contention under parallel runs.
- LMDB — performant but complex; overkill for file-level granularity.

### Decision 3: Invalidation includes Pretender version AND plugin version

**Why:** A Pretender upgrade may change metric computation; a plugin upgrade may change CST shapes.
The cache key is `sha256(file_content) || pretender_version || plugin_version`. Entries for old
versions are simply never hit and are pruned by the age/size policy.

### Decision 4: Serialization format — `bincode` (msgpack fallback)

**Why:** `bincode` is already present in the Rust ecosystem for fast, compact binary serialization.
If cross-platform portability issues arise with `bincode`, the fallback is `rmp-serde` (MessagePack).
JSON is explicitly rejected for cache entries due to ~3× size overhead.

### Decision 5: `<repo-id>` derived from git remote URL hash

**Why:** Isolates caches from different repositories that happen to share a `~/.cache/pretender`
directory. If no remote is configured, fall back to the absolute path of the git root, SHA-256'd.

## Risks / Trade-offs

- **Risk: Cache corruption** → Mitigation: checksum each cache entry header; on deserialization
  failure, treat as cache miss and overwrite.
- **Risk: Disk exhaustion** → Mitigation: size cap enforced at start of every `check` run.
- **Risk: Parallel write races** → Mitigation: writes use a `.tmp` → `rename` atomic pattern;
  concurrent readers always see either the old or new complete entry, never a partial one.

## Migration Plan

Cache is opt-in via `[cache] enabled = true` (default: `true` once the feature ships). No existing
config keys are changed. On first run, the cache directory is created automatically.

## Open Questions

- Should `pretender check` print a cache hit-rate summary with `--verbose`? (Proposed: yes, on stderr.)
- Should `import` validate entry checksums before accepting? (Proposed: yes, reject corrupt entries.)
