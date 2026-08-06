#!/usr/bin/env bash
# Run every project-convention gate. Single entry point for humans and CI:
# conventions live in scripts/gates/, never in CI YAML and never in prose only.
#
#   ./scripts/gates/run-all.sh          # all gates
#   ./scripts/gates/run-all.sh examples # one gate by name
#
# See docs/gates.md for what each gate replaces and why it exists.
set -uo pipefail

GATES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$GATES_DIR/../.." && pwd)"
cd "$REPO_ROOT"

if [ $# -gt 0 ]; then
  gates=("$@")
else
  gates=(workflows examples syntax-canon error-codes secrets budgets changelog)
fi

failed=()
for gate in "${gates[@]}"; do
  script="$GATES_DIR/$gate.sh"
  if [ ! -x "$script" ]; then
    echo "gate '$gate' not found at $script" >&2
    failed+=("$gate")
    continue
  fi
  echo "── gate: $gate ──────────────────────────────────────────"
  if "$script"; then
    echo "   PASS: $gate"
  else
    echo "   FAIL: $gate"
    failed+=("$gate")
  fi
  echo
done

if [ ${#failed[@]} -gt 0 ]; then
  echo "RED — ${#failed[@]} gate(s) failed: ${failed[*]}"
  echo "A red gate means the work is not done. Fix the cause, not the gate."
  exit 1
fi

echo "GREEN — all ${#gates[@]} gate(s) passed."
