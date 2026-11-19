# Corpus calibration for `excessive_fields_max` (ticket pretender-uwz)

Decision input for pinning pretender's default `excessive_fields_max`
(the data-model field-count check, flags a type when `fields >= threshold`).
This lane belongs to the openspec change `add-data-model-checks`.

## Files

| File | Role |
|---|---|
| `corpus-manifest.toml` | 15 pinned OSS repos across the languages pretender captures types for (rust, go, python, java, csharp, typescript), all permissive licenses |
| `fetch-corpus.sh` | Shallow-fetch each manifest entry into a local cache; resumable, credential-free |
| `calibrate-fields.sh` | Run the local pretender binary over the cache per threshold, emit a markdown distribution table + raw JSON per run |

## Why the manifest is TOML, not JSON

Pretender's own config surface is TOML (`pretender.toml`), TOML carries the
per-entry and schema comments a corpus needs for review, and parsing needs
only the python3 stdlib (`tomllib`, >= 3.11) — which both scripts already
require for JSON aggregation. No new tool dependency.

## Usage

```sh
# 1. Fetch the corpus (network, ~minutes; resumable — re-run skips fetched pins)
scripts/calibration/fetch-corpus.sh --check-licenses

# 2. Run the calibration (offline; scans only the cache)
scripts/calibration/calibrate-fields.sh
```

Outputs land under `./.calibration-cache/results/`:

- `summary.md` — distribution table, rows = language, columns = threshold
  (default `7,10,15,20,25`), plus run header and pointers to raw JSON
- `<repo>/threshold-<t>.json` — the raw pretender JSON report for that run
- `<repo>/pin.txt` — manifest pin the run was made against (a bumped pin
  invalidates older results visibly)
- `failures.log` — stderr tails of any run that produced no JSON

`fetch-corpus.sh` extras: `--dry-run` (plan only, no network), `--repos a,b`
(subset), `--check-licenses` (greps checked-out `LICENSE*`/`COPYING*` files
against the manifest's SPDX family; a tripwire, not a verdict).
`calibrate-fields.sh` extras: `--thresholds`, `--repos`, `--binary`,
`--skip-build` (fails instead of running `cargo build` when the binary is
missing).

### Cache dir hygiene

The default cache `.calibration-cache/` (and its `results/`) are throwaway
local state — git-ignore it in your local checkout. These scripts
deliberately do not touch `.gitignore`; add the entry yourself if you want
it permanent, and never commit the cache.

### Smoke test (no network, no git, no cargo)

`calibrate-fields.sh --fixtures-dir DIR` treats each immediate
subdirectory of `DIR` as a repo. Verified end-to-end against a two-language
fixture (rust struct with 12/8/6 fields; python class with 9 and 4
annotated attributes) at thresholds `5,7,10,13`:

| language | 5 | 7 | 10 | 13 |
|---|---:|---:|---:|---:|
| rust | 3 | 2 | 1 | 0 |
| python | 1 | 1 | 0 | 0 |
| **all** | 4 | 3 | 1 | 0 |

(`fields >= threshold` semantics: the 6-field struct flags at 5 but not 7;
the 12-field struct flags at 10 but not 13.) Raw JSON per run is written
next to the table, so any count is auditable.

## Why the current default (10) is provisional

The shipped default of 10 matches no established tool; neighbors bracket it
from both sides with no consensus:

| Tool | Field-count limit |
|---|---|
| pylint (`max-attributes`) | 7 |
| PMD (`TooManyFields`) | 15 |
| NDepend | 15 |
| SonarQube (`S1104`-family class field rules) | 20 |

So 10 is an unvalidated midpoint. The corpus distribution table is the
evidence for choosing a default that flags a defensible tail of real
production types instead of an arbitrary slice.

## HITL sign-off

Running the full corpus (network fetch + minutes-scale scans) and pinning a
new default are human decisions. Calibration output feeds an openspec
proposal; the default itself changes only through that proposal's review —
never by editing `pretender.toml` defaults directly from this lane.

## Manifest pin and license policy

Pins are stable released tags; a pin that does not resolve fails that
repo's fetch loudly — fix the pin in `corpus-manifest.toml` rather than
working around the script. License values were transcribed from each
repo's `LICENSE*` files at the pinned tag; re-verify on pin bumps
(`--check-licenses` gives a coarse first pass).
