#!/usr/bin/env bash
# Gate: every example is analyzable and canonically formatted.
#
# The example set is discovered by glob, never listed by hand. A hand-written
# list in CI YAML is a convention that rots the moment someone adds a file —
# exactly the kind of agreement a machine has to hold, not a reviewer.
source "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

omni="$(resolve_omni)" || exit 1
status=0
count=0

for spec in examples/*.omni; do
  [ -e "$spec" ] || continue
  count=$((count + 1))
  if ! out="$("$omni" check "$spec" 2>&1)"; then
    fail "omni check failed: $spec"
    echo "$out" | sed 's/^/       /'
    status=1
  fi
done

if [ "$count" -eq 0 ]; then
  fail "no examples found under examples/ — the gate is checking nothing"
  exit 1
fi

# The formatter is idempotent (roadmap task 15); an unformatted example means
# the canonical layout drifted.
if ! out="$("$omni" fmt --check examples/ 2>&1)"; then
  fail "omni fmt --check failed — run: omni fmt examples/"
  echo "$out" | sed 's/^/       /'
  status=1
fi

note "$count example(s) checked and formatted"
exit $status
