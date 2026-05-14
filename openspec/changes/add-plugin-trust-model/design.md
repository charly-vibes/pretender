## Context

Pretender plugins are loaded from arbitrary URLs. Any plugin installed via `pretender plugins add <url>` executes as native code (`.so`/`.dylib`/`.dll`) or runs an external command (`command` spec). Without integrity checking, a compromised upstream rev or a MITM can silently replace plugin code. This design adds a lock-file-first trust model analogous to `Cargo.lock` / `package-lock.json`, augmented by a curated registry.

Stakeholders: individual developers (want frictionless install), CI operators (want reproducible, tamper-evident runs), security teams (want supply-chain guarantees).

## Goals / Non-Goals

- Goals:
  - Pin every installed plugin to an immutable (rev, SHA-256) tuple
  - Give CI a single flag (`--frozen-plugins`) to enforce the pinned state
  - Provide a curated registry so common language plugins can be installed without URL trust warnings
  - Keep the UX for registry installs as simple as the current `add <name>` flow
- Non-Goals:
  - WASM sandbox (deferred — noted as future layer in spec)
  - Code review of plugins — we only attest integrity, not safety
  - Managing lock file conflicts across team members (treat lock file as commitable, team syncs it)

## Decisions

- **Decision**: Use TOML for the lock file to stay consistent with `pretender.toml`.
  - Alternatives: JSON (machine-friendly but verbose), binary (not human-readable). TOML wins on consistency.
- **Decision**: SHA-256 for artifact hashing. Widely supported, no known pre-image attacks at current digest sizes, matches tree-sitter grammar pinning convention already used.
- **Decision**: `--i-trust-this` flag (instead of `--force` or `--unsafe`) to make the security tradeoff explicit in scripts.
- **Decision**: Registry signatures use minisign (ed25519). Small dependency, auditable, used by Zig and other tool ecosystems.
  - Alternatives: GPG (too heavy), TUF (correct but complex). Minisign is the minimum viable solution.
- **Decision**: Metric plugin (command) hashes the resolved binary path via `which <cmd>` at install time. Commands with embedded arguments hash the binary only.
- **Decision**: Lock file is created in the project root (alongside `pretender.toml`), not in `~/.config/pretender/`. This enables per-project pinning and makes the lock commitable.

## Risks / Trade-offs

- Lock file grows stale when users manually copy plugins → `pretender plugins verify` provides the fix path
- Curated registry is a single point of trust → mitigated by publishing registry index as a signed, content-addressed artifact; registry URL is configurable
- `--frozen-plugins` breaks on first use for projects without a lock file → clear error message directs users to run `pretender plugins add` to generate the lock

## Migration Plan

1. Existing installations have no lock file. On first run of `pretender plugins verify` or `pretender check --frozen-plugins`, emit an error explaining how to generate the lock: `pretender plugins lock-generate`.
2. `pretender plugins lock-generate` scans `~/.config/pretender/` and re-derives hashes, writing the lock file.
3. Teams commit the lock file; CI adds `--frozen-plugins` to `pretender check`.

## Open Questions

- Should the lock file be stored at project root or `~/.config/pretender/`? Decision above is project root; revisit if multi-project installs prove cumbersome.
- Registry URL: hardcode a default, make it configurable via `[plugins] registry_url`?
