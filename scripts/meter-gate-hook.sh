#!/bin/sh
# Meter for pretender-15w: verifies the gate path stops an agent commit.
#
# Scenario (AFK-verifiable):
#   1. seed a scratch repo with a function violating a default threshold
#      (params 6 > 4) and gate-mode config
#   2. install the pre-commit hook via `pretender hooks install`
#   3. `git commit` the violation
#   4. assert the commit is BLOCKED and the output names the function + rule
#
# GIT_CONFIG_GLOBAL/SYSTEM are neutralized so machine-level core.hooksPath
# overrides (e.g. lefthook shims) cannot intercept the hook under test.
#
# Usage: PRETENDER_BIN=/path/to/pretender scripts/meter-gate-hook.sh
# Exit 0 = gate enforcement verified; non-zero = gate did not stop the commit.
set -u

if [ -z "${PRETENDER_BIN:-}" ] || [ ! -x "$PRETENDER_BIN" ]; then
  echo "meter: set PRETENDER_BIN to the pretender binary under test" >&2
  exit 2
fi
export PATH="$(dirname "$PRETENDER_BIN"):$PATH"

SCRATCH="$(mktemp -d /tmp/pretender-meter-XXXXXX)"
trap 'rm -rf "$SCRATCH"' EXIT
REPO="$SCRATCH/repo"
mkdir -p "$REPO"
cd "$REPO" || exit 2

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
git init -q .
git config user.email meter@pretender.local
git config user.name meter

cat > pretender.toml <<'EOF'
[pretender]
mode = "gate"
EOF

cat > viol.py <<'EOF'
def too_many_params(a, b, c, d, e, f):
    return a
EOF

"$PRETENDER_BIN" hooks install || exit 2
git add -A

OUT="$(git commit -m seed 2>&1)"
STATUS=$?

if [ "$STATUS" -eq 0 ]; then
  echo "meter: FAIL — gate commit succeeded; gate did not stop the violation"
  echo "$OUT"
  exit 1
fi

echo "$OUT" | grep -q "VIOLATION" || {
  echo "meter: FAIL — commit blocked but no VIOLATION verdict in output:"
  echo "$OUT"
  exit 1
}
echo "$OUT" | grep -q "too_many_params" || {
  echo "meter: FAIL — VIOLATION shown but function name missing from output"
  echo "$OUT"
  exit 1
}

echo "meter: PASS — gate blocked the violating commit and named the function + rule"
exit 0
