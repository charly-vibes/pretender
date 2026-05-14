## ADDED Requirements

### Requirement: Cache CLI Subcommands

The system SHALL provide a `pretender cache` subcommand group with the following sub-subcommands:
`export`, `import`, `clear`, and `status`. Each sub-subcommand MUST be accessible as
`pretender cache <subcommand> [args]`.

#### Scenario: cache export writes archive

- **WHEN** `pretender cache export <path>` is invoked
- **THEN** the cache is serialized to a `.tar.zst` archive at `<path>` and the command exits with code 0

#### Scenario: cache import merges archive

- **WHEN** `pretender cache import <path>` is invoked with an existing archive
- **THEN** entries are merged into the local cache and the command reports the number of entries imported

#### Scenario: cache clear removes all entries

- **WHEN** `pretender cache clear` is invoked
- **THEN** all files under the cache directory are deleted and the index is reset

#### Scenario: cache status reports cache state

- **WHEN** `pretender cache status` is invoked
- **THEN** the command prints total size, entry count, and the age of the oldest entry to stdout

---

### Requirement: Cache Configuration Table

The system SHALL support a `[cache]` table in `pretender.toml` with the following keys:

| Key            | Type    | Default                   | Description                              |
|----------------|---------|---------------------------|------------------------------------------|
| `enabled`      | boolean | `true`                    | Enable or disable the cache globally     |
| `max_age_days` | integer | `30`                      | Entries older than this are pruned       |
| `max_size_gb`  | float   | `1`                       | Total cache size ceiling in gigabytes    |
| `path`         | string  | `"~/.cache/pretender"`    | Root directory for all repository caches |

The configured `path` is a cache root; Pretender SHALL append a stable `<repo-id>` subdirectory for the current repository before reading or writing entries. The `PRETENDER_CACHE_DIR` environment variable MUST override the `path` key when set and is also treated as a cache root.
When `enabled = false`, the engine MUST skip all cache reads and writes.

#### Scenario: Cache disabled skips all cache I/O

- **WHEN** `[cache] enabled = false` is set in `pretender.toml`
- **THEN** `pretender check` runs without reading or writing any cache files

#### Scenario: Custom path overrides default

- **WHEN** `[cache] path = "/tmp/my-cache"` is set
- **THEN** all cache files for the current repository are read from and written to `/tmp/my-cache/<repo-id>`

#### Scenario: Environment variable overrides config path

- **WHEN** the environment variable `PRETENDER_CACHE_DIR=/ci/cache` is set
- **THEN** it takes precedence over the `path` key in `pretender.toml` and repository cache files are stored under `/ci/cache/<repo-id>`
