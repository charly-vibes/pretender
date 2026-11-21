#!/usr/bin/env bash
# Calibration runner for `excessive_fields_max` (ticket pretender-uwz).
#
# For every cached corpus repo (scripts/calibration/corpus-manifest.toml,
# fetched by fetch-corpus.sh) and every threshold in {7,10,15,20,25}, run
# the local pretender binary with a generated temp pretender.toml, collect
# the `excessive_fields` findings per language, and emit a markdown
# distribution table (rows = language, columns = threshold) plus the raw
# JSON report for each (repo, threshold) run.
#
# The script never reads the network; it only scans the cache dir — or a
# --fixtures-dir of plain directories (no git required), which is how it
# is smoke-tested against tiny fake repos with known field counts.
#
# Semantics: pretender flags a type when fields >= threshold (>=, not >),
# so counts are monotonically non-increasing in the threshold. The app
# threshold is pinned in the generated config and the test-role threshold
# is pinned to the same value, so every role is counted uniformly.
#
# Usage: scripts/calibration/calibrate-fields.sh [--manifest PATH]
#          [--cache-dir DIR] [--results-dir DIR] [--thresholds 7,10,15,20,25]
#          [--repos name1,name2] [--fixtures-dir DIR] [--binary PATH]
#          [--skip-build] [-h|--help]
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/calibration/calibrate-fields.sh [options]

Options:
  --manifest PATH     Corpus manifest (default: corpus-manifest.toml next to this script)
  --cache-dir DIR     Cache root fed by fetch-corpus.sh (default: ./.calibration-cache)
  --results-dir DIR   Output root (default: <cache-dir>/results; keep it out of git)
  --thresholds LIST   Comma-separated thresholds (default: 7,10,15,20,25)
  --repos a,b,c       Only run the named repos (default: all cached manifest entries)
  --fixtures-dir DIR  Smoke-test mode: treat each immediate subdirectory of DIR
                      as a repo; ignores --manifest/--cache-dir (no git needed)
  --binary PATH       Pretender binary (default: <repo-root>/target/debug/pretender)
  --skip-build        Fail instead of running `cargo build` when the binary is missing
  -h, --help          This help

Outputs under --results-dir:
  summary.md                    markdown distribution table + run header
  <repo>/threshold-<t>.json     raw pretender JSON report per run
  <repo>/pin.txt                manifest pin the run was made against (fixtures: "fixture")
  failures.log                  stderr tails of runs that produced no JSON
Exit 0 iff every (repo, threshold) run produced a JSON report.
EOF
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
manifest="$script_dir/corpus-manifest.toml"
cache_dir="./.calibration-cache"
results_dir=""
thresholds="7,10,15,20,25"
repos_filter=""
fixtures_dir=""
binary=""
skip_build=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest) manifest="$2"; shift 2 ;;
    --cache-dir) cache_dir="$2"; shift 2 ;;
    --results-dir) results_dir="$2"; shift 2 ;;
    --thresholds) thresholds="$2"; shift 2 ;;
    --repos) repos_filter="$2"; shift 2 ;;
    --fixtures-dir) fixtures_dir="$2"; shift 2 ;;
    --binary) binary="$2"; shift 2 ;;
    --skip-build) skip_build=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "calibrate-fields: unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

command -v python3 >/dev/null 2>&1 || { echo "calibrate-fields: python3 is required" >&2; exit 2; }

# Threshold list: positive integers, comma-separated.
if ! grep -Eq '^[0-9]+(,[0-9]+)*$' <<<"$thresholds"; then
  echo "calibrate-fields: --thresholds must be comma-separated integers: $thresholds" >&2
  exit 2
fi
IFS=',' read -r -a threshold_list <<<"$thresholds"
for t in "${threshold_list[@]}"; do
  if [[ "$t" -eq 0 ]]; then
    echo "calibrate-fields: threshold 0 disables the check in pretender — use >= 1" >&2
    exit 2
  fi
done

# Binary: --binary wins; else <repo-root>/target/debug/pretender; build if
# missing and allowed. (cargo runs only here, never during the runs.)
if [[ -z "$binary" ]]; then
  binary="$repo_root/target/debug/pretender"
fi
if [[ ! -x "$binary" ]]; then
  if [[ $skip_build -eq 1 ]]; then
    echo "calibrate-fields: binary missing and --skip-build given: $binary" >&2
    exit 2
  fi
  command -v cargo >/dev/null 2>&1 || {
    echo "calibrate-fields: binary missing and cargo unavailable: $binary" >&2
    exit 2
  }
  echo "calibrate-fields: building pretender (cargo build, once)"
  (cd "$repo_root" && cargo build --quiet) || {
    echo "calibrate-fields: cargo build failed" >&2
    exit 2
  }
fi

# Repo discovery -------------------------------------------------------------
# fixtures mode: every immediate subdirectory of --fixtures-dir is a repo.
# manifest mode: manifest entries that have been fetched into the cache.
declare -a repo_names=()
declare -A repo_dirs=()
if [[ -n "$fixtures_dir" ]]; then
  [[ -d "$fixtures_dir" ]] || { echo "calibrate-fields: --fixtures-dir not a directory: $fixtures_dir" >&2; exit 2; }
  for d in "$fixtures_dir"/*/; do
    [[ -d "$d" ]] || continue
    name="$(basename "$d")"
    repo_names+=("$name")
    repo_dirs["$name"]="$(cd "$d" && pwd)"
  done
  if [[ ${#repo_names[@]} -eq 0 ]]; then
    echo "calibrate-fields: no fixture subdirectories under $fixtures_dir" >&2
    exit 2
  fi
else
  [[ -f "$manifest" ]] || { echo "calibrate-fields: manifest not found: $manifest" >&2; exit 2; }
  manifest_rows() {
    python3 - "$manifest" "$repos_filter" "$cache_dir" <<'PY'
import sys
import tomllib

manifest, repos_filter, cache_dir = sys.argv[1], sys.argv[2], sys.argv[3]
wanted = None
if repos_filter:
    wanted = {r.strip() for r in repos_filter.split(",") if r.strip()}
with open(manifest, "rb") as fh:
    data = tomllib.load(fh)
if data.get("schema_version") != 1:
    sys.exit(f"unsupported manifest schema_version={data.get('schema_version')!r}")
for entry in data.get("repo", []):
    if wanted is not None and entry["name"] not in wanted:
        continue
    print(entry["name"], entry.get("pin", ""), sep="|")
PY
  }
  cached_rows="$(manifest_rows)"
  if [[ -z "$cached_rows" ]]; then
    echo "calibrate-fields: no repos selected (check --repos against the manifest)" >&2
    exit 2
  fi
  while IFS='|' read -r name _pin; do
    [[ -n "$name" ]] || continue
    dir="$cache_dir/$name"
    if [[ ! -f "$dir/.pretender-pin" ]]; then
      echo "calibrate-fields: skipping $name — not fetched yet (run fetch-corpus.sh first)" >&2
      continue
    fi
    repo_names+=("$name")
    repo_dirs["$name"]="$(cd "$dir" && pwd)"
  done <<<"$cached_rows"
  if [[ ${#repo_names[@]} -eq 0 ]]; then
    echo "calibrate-fields: no cached repos to run (see skipped warnings above)" >&2
    exit 2
  fi
fi

if [[ -z "$results_dir" ]]; then
  results_dir="$cache_dir/results"
fi
mkdir -p "$results_dir"
failures_log="$results_dir/failures.log"
: >"$failures_log"

pretender_version="$("$binary" --version 2>/dev/null | head -1 || echo unknown)"
run_date="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# One generated config per run, in a throwaway cwd, so no repo and no
# working tree is ever modified. App + test roles pinned to the same
# threshold so the per-language counts are uniform across roles.
run_one() {
  local name="$1" repo_dir="$2" t="$3"
  local scratch json_file status
  scratch="$(mktemp -d "${TMPDIR:-/tmp}/calibrate-fields.XXXXXX")"
  {
    printf '# Generated by scripts/calibration/calibrate-fields.sh - do not edit.\n'
    printf '[thresholds]\nexcessive_fields_max = %s\n\n' "$t"
    printf '[thresholds.test]\nexcessive_fields_max = %s\n' "$t"
  } >"$scratch/pretender.toml"
  mkdir -p "$results_dir/$name"
  json_file="$results_dir/$name/threshold-$t.json"
  status=0
  # Gate mode exits non-zero on findings; the JSON report is written either
  # way, so only a missing/unparseable report counts as a failure.
  (cd "$scratch" && "$binary" check "$repo_dir" --format json --output "$json_file") \
    >"$scratch/stdout.log" 2>"$scratch/stderr.log" || status=$?
  if [[ ! -s "$json_file" ]]; then
    {
      echo "repo=$name threshold=$t exit=$status"
      tail -5 "$scratch/stderr.log"
      echo "---"
    } >>"$failures_log"
    echo "run: $name t=$t FAILED (exit=$status; see failures.log)"
    rm -rf "$scratch"
    return 1
  fi
  echo "run: $name t=$t ok (exit=$status)"
  rm -rf "$scratch"
}

# Per (repo, threshold) runs; keep going after individual failures.
run_failures=0
for name in "${repo_names[@]}"; do
  repo_dir="${repo_dirs[$name]}"
  mkdir -p "$results_dir/$name"
  if [[ -f "$repo_dir/.pretender-pin" ]]; then
    cat "$repo_dir/.pretender-pin" >"$results_dir/$name/pin.txt" 2>/dev/null \
      || printf 'fixture\n' >"$results_dir/$name/pin.txt"
  else
    printf 'fixture\n' >"$results_dir/$name/pin.txt"
  fi
  for t in "${threshold_list[@]}"; do
    if ! run_one "$name" "$repo_dir" "$t"; then
      run_failures=$((run_failures + 1))
    fi
  done
done

# Aggregate: markdown table (rows = language, cols = threshold) from the
# raw JSON reports; per-language attribution by file extension.
python3 - "$results_dir" "$thresholds" "$repo_root" "$pretender_version" "$run_date" <<'PY' || aggregate_status=$?
import json
import os
import sys

results_dir, thresholds_arg, repo_root, version, run_date = sys.argv[1:6]
thresholds = [int(x) for x in thresholds_arg.split(",")]

EXT_LANG = {
    ".rs": "rust", ".go": "go", ".py": "python", ".java": "java",
    ".cs": "csharp", ".ts": "typescript", ".tsx": "typescript",
    ".mts": "typescript", ".cts": "typescript", ".js": "javascript",
    ".jsx": "javascript", ".mjs": "javascript", ".cjs": "javascript",
    ".rb": "ruby", ".c": "c", ".h": "c", ".cpp": "cpp", ".cc": "cpp",
    ".cxx": "cpp", ".hpp": "cpp", ".hh": "cpp",
}
LANG_ORDER = ["rust", "go", "python", "java", "csharp", "typescript"]


def lang_for(path):
    return EXT_LANG.get(os.path.splitext(path)[1].lower(), "other")


repos = sorted(
    d for d in os.listdir(results_dir)
    if os.path.isdir(os.path.join(results_dir, d))
)
# counts[lang][threshold] = findings; failed[repo][threshold] = True
counts, failed, pins = {}, {}, {}
runs_total = runs_ok = 0
for repo in repos:
    repo_dir = os.path.join(results_dir, repo)
    pin_file = os.path.join(repo_dir, "pin.txt")
    if os.path.isfile(pin_file):
        pins[repo] = open(pin_file).read().strip() or "unknown"
    for t in thresholds:
        runs_total += 1
        report = os.path.join(repo_dir, f"threshold-{t}.json")
        try:
            with open(report) as fh:
                data = json.load(fh)
        except (OSError, ValueError):
            failed.setdefault(repo, set()).add(t)
            continue
        runs_ok += 1
        for file_report in data.get("data", {}).get("files", []):
            for violation in file_report.get("file_violations", []):
                if violation.get("metric") != "excessive_fields":
                    continue
                lang = lang_for(file_report.get("path", ""))
                counts.setdefault(lang, {}).setdefault(t, 0)
                counts[lang][t] += 1

langs = [l for l in LANG_ORDER if l in counts]
langs += sorted(l for l in counts if l not in LANG_ORDER)

lines = []
lines.append("# excessive_fields_max calibration distribution")
lines.append("")
lines.append(f"- Generated: {run_date} UTC")
lines.append(f"- Binary: `{version}`")
lines.append(f"- Corpus: {len(repos)} repo(s): " + ", ".join(
    f"{r} @ {pins.get(r, 'unknown')}" for r in repos))
lines.append(f"- Thresholds: {', '.join(str(t) for t in thresholds)} (finding iff fields >= threshold; "
             "counts are `excessive_fields` findings summed over all roles)")
lines.append(f"- Runs: {runs_ok}/{runs_total} produced JSON "
             f"(failures: `failures.log`)" if failed else f"- Runs: {runs_ok}/{runs_total} produced JSON")
lines.append("")
header = "| language | " + " | ".join(str(t) for t in thresholds) + " |"
rule = "|---|" + "---:|" * len(thresholds)
lines += [header, rule]

for lang in langs:
    row = [lang]
    for t in thresholds:
        row.append(str(counts.get(lang, {}).get(t, 0)))
    lines.append("| " + " | ".join(row) + " |")
row = ["**all**"]
for t in thresholds:
    total = sum(counts.get(l, {}).get(t, 0) for l in langs)
    row.append(str(total))
lines.append("| " + " | ".join(row) + " |")
lines.append("")
lines.append("Raw JSON per run: `<repo>/threshold-<t>.json`; "
             "re-aggregate without re-running via this script's python block.")
for repo, ts in sorted(failed.items()):
    lines.append(f"- run failure: {repo} threshold(s) {', '.join(str(t) for t in sorted(ts))}")

out_path = os.path.join(results_dir, "summary.md")
with open(out_path, "w") as fh:
    fh.write("\n".join(lines) + "\n")
print("\n".join(lines))
sys.exit(1 if failed else 0)
PY
if [[ ${aggregate_status:-0} -ne 0 ]]; then
  echo "calibrate-fields: aggregation completed with run failures (see $failures_log)" >&2
  exit 1
fi
echo "calibrate-fields: summary written to $results_dir/summary.md"
