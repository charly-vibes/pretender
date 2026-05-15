## 1. Lock File

- [ ] 1.1 Define `pretender.plugins.lock` TOML schema (plugin name, kind, source, rev, artifact_sha256, installed_at)
- [ ] 1.2 Implement lock file read/write in plugin loader
- [ ] 1.3 Populate lock entry on `pretender plugins add`
- [ ] 1.4 Include command hash for metric plugins (TOML `command` spec)
- [ ] 1.5 Implement `pretender plugins lock-generate` for existing installations

## 2. Verify Command

- [ ] 2.1 Implement `pretender plugins verify` — re-download artifact and compare SHA-256
- [ ] 2.2 Exit non-zero if any mismatch; print which plugin(s) failed
- [ ] 2.3 Print `ok` for each plugin that matches the lock

## 3. URL Add Warning

- [ ] 3.1 Detect non-registry installs (source has URL scheme: `github:`, `https:`, path)
- [ ] 3.2 Print warning: "unverified code — review source before trusting"
- [ ] 3.3 Require `--i-trust-this` flag when stdin is not a TTY
- [ ] 3.4 Interactive: prompt "Trust this plugin? [y/N]"

## 4. Registry Install

- [ ] 4.1 Define curated registry manifest format and default registry URL
- [ ] 4.2 Implement registry lookup for bare name installs (`pretender plugins add elixir`)
- [ ] 4.3 Verify registry signature before installing
- [ ] 4.4 Populate lock file from registry metadata (rev + artifact SHA-256)

## 5. Frozen Plugins Flag

- [ ] 5.1 Add `--frozen-plugins` flag to `pretender check`
- [ ] 5.2 On startup with `--frozen-plugins`: read lock file and verify all installed plugins match
- [ ] 5.3 Exit with non-zero code and actionable message if any mismatch

## 6. Tests

- [ ] 6.1 Unit test: lock file round-trip serialization
- [ ] 6.2 Integration test: `plugins verify` detects tampered artifact
- [ ] 6.3 Integration test: `--frozen-plugins` fails on missing lock entry
- [ ] 6.4 Integration test: bare name install from registry populates lock
- [ ] 6.5 Integration test: `plugins lock-generate` writes lock entries for already-installed plugins
