## 1. Cache Storage Layer

- [ ] 1.1 Implement `src/cache/mod.rs` with `CacheKey` struct (file content SHA-256, Pretender version, plugin version)
- [ ] 1.2 Implement `CacheStore::lookup(key) -> Option<CachedResult>` and `CacheStore::write(key, result)`
- [ ] 1.3 Implement `index.json` read/write with path → hash + last-used timestamp
- [ ] 1.4 Ensure cache directory is `~/.cache/pretender/<repo-id>/metrics/` by default

## 2. Engine Integration

- [ ] 2.1 Thread `CacheStore` into the check pipeline before CST parsing
- [ ] 2.2 On cache hit: deserialize `Module` + metric results; skip parse and compute
- [ ] 2.3 On cache miss: compute normally, then serialize and write to cache
- [ ] 2.4 Update `index.json` last-used timestamp on every cache hit

## 3. Pruning

- [ ] 3.1 Implement `CacheStore::prune()` — remove entries older than `max_age_days`
- [ ] 3.2 Enforce `max_size_gb` ceiling: evict LRU entries until under budget
- [ ] 3.3 Run pruning lazily at start of `pretender check` (non-blocking, best-effort)

## 4. CLI Subcommands

- [ ] 4.1 Implement `pretender cache export <path>` — writes a `.tar.zst` archive of the cache directory
- [ ] 4.2 Implement `pretender cache import <path>` — extracts archive, merges entries into local cache
- [ ] 4.3 Implement `pretender cache clear` — removes all entries from the cache directory
- [ ] 4.4 Implement `pretender cache status` — prints current size, entry count, and oldest entry

## 5. Configuration

- [ ] 5.1 Add `[cache]` table parsing in config loader
- [ ] 5.2 Wire `enabled`, `max_age_days`, `max_size_gb`, `path` to `CacheStore` construction
- [ ] 5.3 Honor `PRETENDER_CACHE_DIR` environment variable as an override for `path`

## 6. Tests

- [ ] 6.1 Unit tests: hash stability across identical content, version bump invalidates entry
- [ ] 6.2 Integration test: cold run writes cache; warm run hits cache; mutated file misses cache
- [ ] 6.3 Test pruning by age and by size cap
- [ ] 6.4 Test `export` + `import` round-trip
