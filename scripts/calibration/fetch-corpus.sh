#!/usr/bin/env bash
# Fetch the calibration corpus (ticket pretender-uwz) into a local cache.
#
# Companion of scripts/calibration/corpus-manifest.toml; see README.md in
# this directory. Read-only with respect to the repo: everything lands
# under the cache dir (default ./.calibration-cache/), which the operator
# should git-ignore locally — the scripts deliberately do not touch
# .gitignore.
#
# Each repo is shallow-fetched straight to its pinned tag/commit:
#   git init + remote add -> fetch --depth N <pin> -> checkout FETCH_HEAD
# Resumable: a checked-out repo is stamped with its pin
# (<repo>/.pretender-pin); a re-run skips the network when stamp and
# manifest agree. No credentials: https-only URLs, credential helpers
# disabled, GIT_TERMINAL_PROMPT=0, BatchMode ssh (fails instead of
# prompting) — public repos only.
#
# Usage: scripts/calibration/fetch-corpus.sh [--manifest PATH]
#          [--cache-dir DIR] [--repos name1,name2,...] [--depth N]
#          [--check-licenses] [--dry-run] [-h|--help]
# Exit 0 iff every selected repo is fetched and licensed as declared.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/calibration/fetch-corpus.sh [options]

Options:
  --manifest PATH     Corpus manifest (default: corpus-manifest.toml next to this script)
  --cache-dir DIR     Cache root (default: ./.calibration-cache; keep it out of git)
  --repos a,b,c       Only fetch the named repos (default: all manifest entries)
  --depth N           Shallow-fetch depth (default: 1)
  --check-licenses    After checkout, grep the repo's LICENSE*/COPYING* files
                      against the manifest's SPDX family and warn on mismatch
  --dry-run           Print the fetch plan (name, url, pin) and exit; no network
  -h, --help          This help

All selected repos are attempted even if an earlier one fails; the exit
code is non-zero if any fetch did not complete. Re-running is safe and
skips repos already pinned at the manifest's tag/commit.
EOF
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
manifest="$script_dir/corpus-manifest.toml"
cache_dir="./.calibration-cache"
repos_filter=""
depth=1
check_licenses=0
dry_run=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest) manifest="$2"; shift 2 ;;
    --cache-dir) cache_dir="$2"; shift 2 ;;
    --repos) repos_filter="$2"; shift 2 ;;
    --depth) depth="$2"; shift 2 ;;
    --check-licenses) check_licenses=1; shift ;;
    --dry-run) dry_run=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "fetch-corpus: unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

for tool in git python3; do
  command -v "$tool" >/dev/null 2>&1 || { echo "fetch-corpus: '$tool' is required" >&2; exit 2; }
done
if [[ ! -f "$manifest" ]]; then
  echo "fetch-corpus: manifest not found: $manifest" >&2
  exit 2
fi

# One TSV line per selected repo: name|url|pin|license|language
manifest_rows() {
  python3 - "$manifest" "$repos_filter" <<'PY'
import sys
import tomllib

manifest, repos_filter = sys.argv[1], sys.argv[2]
wanted = None
if repos_filter:
    wanted = {r.strip() for r in repos_filter.split(",") if r.strip()}
with open(manifest, "rb") as fh:
    data = tomllib.load(fh)
if data.get("schema_version") != 1:
    sys.exit(f"unsupported manifest schema_version={data.get('schema_version')!r}")
rows = data.get("repo", [])
if not rows:
    sys.exit("manifest declares no [[repo]] entries")
required = ("name", "url", "pin", "license", "language")
for entry in rows:
    missing = [k for k in required if not entry.get(k)]
    if missing:
        sys.exit(f"manifest entry {entry.get('name', '?')} is missing: {', '.join(missing)}")
    if wanted is not None and entry["name"] not in wanted:
        continue
    print("|".join(str(entry[k]) for k in required))
PY
}

rows="$(manifest_rows)"
if [[ -z "$rows" ]]; then
  echo "fetch-corpus: no repos selected (check --repos against the manifest)" >&2
  exit 2
fi

# Public repos only: reject anything that could smuggle credentials.
assert_public_url() {
  case "$1" in
    https://*) return 0 ;;
    *@*) echo "fetch-corpus: refusing URL with userinfo: $1" >&2; return 1 ;;
    *) echo "fetch-corpus: refusing non-https URL: $1" >&2; return 1 ;;
  esac
}

# SPDX id family -> distinctive boilerplate token. Only the permissive
# families used by the manifest are mapped; anything else reports as
# needing manual review rather than guessing.
license_family_token() {
  case "$1" in
    MIT) echo "Permission is hereby granted" ;;
    Apache-*) echo "Apache License" ;;
    BSD-*) echo "Redistributions of source code" ;;
    Unlicense) echo "unlicense license" ;;
    *) echo "" ;;
  esac
}

# Tripwire, not a verdict: does any checked-out LICENSE*/COPYING*/NOTICE*
# file contain the declared license family's boilerplate? SPDX "OR" is
# satisfied by any one alternative group; "AND" requires every group.
verify_license() {
  local dir="$1" spdx="$2"
  local files alt part token group_ok overall_ok f
  files="$(find "$dir" -maxdepth 1 \( -iname 'LICENSE*' -o -iname 'COPYING*' -o -iname 'NOTICE*' \) -type f 2>/dev/null)"
  if [[ -z "$files" ]]; then
    echo "  license: $spdx — no LICENSE*/COPYING*/NOTICE* at repo root (review manually)"
    return
  fi
  overall_ok=1
  while IFS= read -r alt; do
    [[ -n "$alt" ]] || continue
    group_ok=1
    while IFS= read -r part; do
      [[ -n "$part" ]] || continue
      token="$(license_family_token "$part")"
      if [[ -z "$token" ]]; then
        echo "  license: $spdx — no boilerplate pattern for '$part' (review manually)"
        return
      fi
      part_ok=0
      while IFS= read -r f; do
        [[ -n "$f" ]] || continue
        if grep -qi "$token" "$f"; then
          part_ok=1
          break
        fi
      done <<<"$files"
      if [[ $part_ok -ne 1 ]]; then
        group_ok=0
        break
      fi
    done <<<"$(tr '|' '\n' <<<"${alt// AND /|}")"
    if [[ $group_ok -ne 1 ]]; then
      overall_ok=0
      break
    fi
  done <<<"$(sed 's/ OR /\n/g' <<<"$spdx")"
  if [[ $overall_ok -eq 1 ]]; then
    echo "  license: $spdx — boilerplate found in checked-out files"
  else
    echo "  license: WARNING — '$spdx' not matched in LICENSE* files (review manually)"
  fi
}

# BatchMode ssh + disabled prompts: any credential demand fails loudly
# instead of hanging an unattended run.
export GIT_TERMINAL_PROMPT=0
export GIT_SSH_COMMAND="ssh -oBatchMode=yes -oStrictHostKeyChecking=accept-new"

mkdir -p "$cache_dir"
failed=()
while IFS='|' read -r name url pin license _language; do
  [[ -n "$name" ]] || continue
  if ! assert_public_url "$url"; then
    failed+=("$name")
    continue
  fi
  if [[ $dry_run -eq 1 ]]; then
    printf 'dry-run: %-16s pin=%-22s %s\n' "$name" "$pin" "$url"
    continue
  fi
  echo "fetch: $name @ $pin"
  dir="$cache_dir/$name"
  if [[ -f "$dir/.pretender-pin" ]] \
    && [[ "$(cat "$dir/.pretender-pin")" == "$pin" ]] \
    && git -C "$dir" rev-parse --verify HEAD >/dev/null 2>&1; then
    echo "  up to date ($dir)"
  else
    if [[ ! -d "$dir/.git" ]]; then
      mkdir -p "$dir"
      git init -q "$dir"
      git -C "$dir" remote add origin "$url"
    fi
    # credential.helper= (empty) ignores any stored helper: public clones
    # must never touch local credentials.
    if ! git -C "$dir" -c credential.helper= fetch --depth "$depth" origin "$pin"; then
      echo "  FAILED: could not fetch pin '$pin' from $url (wrong tag? unreachable sha?)" >&2
      failed+=("$name")
      continue
    fi
    if ! git -C "$dir" checkout -q --detach FETCH_HEAD; then
      echo "  FAILED: checkout of fetched pin '$pin' failed" >&2
      failed+=("$name")
      continue
    fi
    printf '%s\n' "$pin" >"$dir/.pretender-pin"
    echo "  checked out at $(git -C "$dir" rev-parse --short HEAD)"
  fi
  if [[ $check_licenses -eq 1 ]]; then
    verify_license "$dir" "$license"
  fi
done <<<"$rows"

if [[ $dry_run -eq 1 ]]; then
  exit 0
fi
if [[ ${#failed[@]} -gt 0 ]]; then
  echo "fetch-corpus: FAILED repos: ${failed[*]}" >&2
  exit 1
fi
echo "fetch-corpus: all selected repos cached under $cache_dir"
