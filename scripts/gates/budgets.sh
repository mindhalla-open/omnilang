#!/usr/bin/env bash
# Gate: the estimated build cost of the reference specs stays inside a budget.
#
# The CI cost-report job posted a table nobody blocks on — and its `ls src/**/*.omni`
# guard never matched anything in this repo, so it reported nothing at all. A
# number without a threshold is a report; a number with a threshold is a gate.
# Ceilings live in scripts/gates/budgets.json.
source "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

omni="$(resolve_omni)" || exit 1
BUDGETS="scripts/gates/budgets.json"

costs=""
for spec in examples/*.omni; do
  [ -e "$spec" ] || continue
  json="$("$omni" plan "$spec" --format json 2>/dev/null)" || {
    fail "omni plan failed: $spec"
    exit 1
  }
  usd="$(printf '%s' "$json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["estimated_cost_usd"])')"
  costs="${costs}${spec} ${usd}"$'\n'
done

printf '%s' "$costs" | python3 scripts/gates/_budgets_check.py "$BUDGETS"
