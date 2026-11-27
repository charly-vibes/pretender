#!/usr/bin/env bash
# tests-pass — pipeline oracle: the relevant test suites must be green.
# Runs the pretender workspace test suite; a failure blocks the gate.
set -o pipefail
cd "$(dirname "$0")/../../.." || exit 1

cargo test 2>&1 | tail -40
status=${PIPESTATUS[0]}
if [ "$status" -ne 0 ]; then
  echo "cargo test failed (exit $status)" >&2
  exit "$status"
fi
