#!/usr/bin/env bash
# Shared helpers for the convention gates.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# Resolve the `omni` binary. Always builds first unless $OMNI is set: a stale
# binary left in target/ makes every gate below it lie in both directions (this
# repo had a months-old target/release/omni that predates `omni fmt`). Cargo is
# a no-op when the tree is unchanged, so the cost is a second and the guarantee
# is that gates test the working tree.
resolve_omni() {
  if [ -n "${OMNI:-}" ]; then echo "$OMNI"; return 0; fi
  cargo build --quiet --bin omni >&2 || return 1
  echo "$REPO_ROOT/target/debug/omni"
}

fail() { echo "   ✗ $*" >&2; }
note() { echo "   · $*"; }
