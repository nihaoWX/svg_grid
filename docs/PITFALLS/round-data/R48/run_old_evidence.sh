#!/bin/bash
# Run every round's evidence script at top level, one process per round.
#
# A round's test.py may re-invoke an EARLIER round's test.py (through
# sys.executable). Left alone that nested run would re-report the earlier round
# as green no matter what it does now, so `shim/python3` (reached via
# `shim/sitecustomize.py`, which rewrites sys.executable) short-circuits a nested
# round ONLY when ran.txt records it as OK from its own top-level run, and fails
# hard otherwise.
#
# Usage:  bash docs/PITFALLS/round-data/R48/run_old_evidence.sh [R8 R9 ...]
#         (no arguments = every round directory that exists)
#
# Writes one line per round to stdout and the same lines to shim/ran.txt.
set -u

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../../.." && pwd)"
SHIM="$HERE/shim"
RAN="$SHIM/ran.txt"
cd "$ROOT" || exit 1

if [ "$#" -gt 0 ]; then
  ROUNDS=("$@")
else
  # Numeric order, not glob order: a round may re-run an *earlier* round, and
  # the shim only vouches for rounds that already ran green at top level.
  ROUNDS=()
  for dir in $(ls -d docs/PITFALLS/round-data/R*/ | sed 's#.*/\(R[0-9]*\)/#\1#' | sort -V); do
    ROUNDS+=("$dir")
  done
fi

: > "$RAN"
failed=0
for round in "${ROUNDS[@]}"; do
  script="docs/PITFALLS/round-data/$round/test.py"
  if [ ! -f "$script" ]; then
    echo "$round MISSING $script"
    failed=$((failed + 1))
    continue
  fi
  output=$(SVG_GRID_EVIDENCE_TOPLEVEL=1 \
           SVG_GRID_EVIDENCE_TOPLEVEL_THIS="$round" \
           SVG_GRID_SHIM_BASELINE_BIN="${SVG_GRID_BIN:-}" \
           SVG_GRID_SHIM_BASELINE_RENDER="${SVG_GRID_RENDER_BIN:-}" \
           SVG_GRID_SHIM_BASELINE_OLD="${SVG_GRID_OLD_BIN:-}" \
           PYTHONPATH="$SHIM${PYTHONPATH:+:$PYTHONPATH}" \
           /usr/bin/python3 "$script" 2>&1)
  code=$?
  if [ "$code" -eq 0 ]; then
    echo "$round OK" | tee -a "$RAN"
  else
    echo "$round FAIL rc=$code" | tee -a "$RAN"
    echo "$output" | tail -6 | sed 's/^/    /'
    failed=$((failed + 1))
  fi
done

echo "== $(( ${#ROUNDS[@]} - failed )) / ${#ROUNDS[@]} rounds green =="
exit $(( failed > 0 ))
