## ADDED Requirements

### Requirement: Content-Addressed Cache Storage

The system SHALL maintain a two-layer content-addressed cache under `~/.cache/pretender/<repo-id>/`.
The cache MUST store serialized `Module` structs and their computed metric results as flat files at
`metrics/<sha256-of-file-content>`. A JSON index at `index.json` MUST record, for each tracked
source path, its current content hash and the epoch timestamp of last use.

#### Scenario: Cold run writes cache entry

- **WHEN** a source file is checked and no cache entry exists for its content hash
- **THEN** the engine computes metrics normally and writes a new cache entry before returning results

#### Scenario: Warm run returns cached metrics

- **WHEN** a source file is checked and a valid cache entry exists for its current content hash, Pretender version, and language-plugin version
- **THEN** the engine deserializes and returns the cached result without re-parsing or recomputing

#### Scenario: File modification invalidates entry

- **WHEN** a source file's content changes between two `pretender check` runs
- **THEN** the old cache entry is not served and new metrics are computed and cached

---

### Requirement: Invalidation Key

The cache invalidation key for each entry MUST be the tuple `(SHA-256 of file content, Pretender version, language-plugin version)`.
An entry MUST NOT be served if any component of the key differs from the current runtime values.

#### Scenario: Pretender version bump invalidates cache

- **WHEN** Pretender is upgraded and a cached entry exists from the previous version
- **THEN** the entry is not served (cache miss) and new metrics are computed

#### Scenario: Plugin version bump invalidates cache

- **WHEN** a language plugin is upgraded and a cached entry exists from the previous plugin version
- **THEN** the entry is not served (cache miss) and new metrics are computed

---

### Requirement: Automatic Cache Pruning

The system SHALL automatically prune cache entries that are older than `max_age_days` (default: 30)
or that cause the total cache size to exceed `max_size_gb` (default: 1). When size pruning is
required, the system MUST evict least-recently-used entries first. Pruning SHALL run at the start of
`pretender check` in a non-blocking best-effort manner.

#### Scenario: Age-based pruning removes stale entries

- **WHEN** a cache entry's last-used timestamp is older than `max_age_days`
- **THEN** the entry is deleted during the next pruning pass

#### Scenario: Size-based pruning evicts LRU entries

- **WHEN** the total cache size exceeds `max_size_gb`
- **THEN** the least-recently-used entries are deleted until the cache is under the size limit

---

### Requirement: Cache Entry Integrity

Each cache entry MUST include a checksum in its header. On deserialization, the system SHALL verify
the checksum. A corrupt or unreadable entry MUST be treated as a cache miss; the system MUST NOT
return partial or invalid metric results from cache.

#### Scenario: Corrupt entry triggers cache miss

- **WHEN** a cache entry exists but its checksum does not match its content
- **THEN** the entry is treated as a miss, metrics are recomputed, and the corrupt file is overwritten

---

### Requirement: Cache Export and Import

The system SHALL provide `pretender cache export <path>` and `pretender cache import <path>` commands
to serialize and deserialize the local cache as a compressed archive, enabling reuse across CI job
steps or machines.

#### Scenario: Export produces transferable archive

- **WHEN** `pretender cache export <path>` is invoked
- **THEN** the cache directory is archived to `<path>` as a `.tar.zst` file containing all current entries

#### Scenario: Import merges entries into local cache

- **WHEN** `pretender cache import <path>` is invoked with a valid archive
- **THEN** entries from the archive are merged into the local cache; existing entries with the same key are kept (local wins)

#### Scenario: Import rejects corrupt archive entries

- **WHEN** `pretender cache import <path>` is invoked and an entry in the archive has an invalid checksum
- **THEN** that entry is skipped with a warning and all other valid entries are imported normally
