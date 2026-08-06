#!/usr/bin/env bash
# Gate: CI workflow files are valid and cannot repeat the 2026-05 outage.
#
# INCIDENT (2026-05-26 → 2026-08-06): commit 39f7bd2 added
#   if: ${{ secrets.ANTHROPIC_API_KEY != '' }}
# to a step. The `secrets` context is not available in `if:`, which is a
# workflow *validation* error: GitHub failed every run in 0s, produced no jobs
# and no logs, and the failure looked like a red dot nobody clicked. Five PRs
# merged with no gates at all during the outage.
#
# This gate is that incident, converted. Per the manifesto: an error is closed
# by a check, not by a promise to be careful.
source "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

status=0
workflows="$(ls .github/workflows/*.yml .github/workflows/*.yaml 2>/dev/null)"

if [ -z "$workflows" ]; then
  fail "no workflow files found — the gate is checking nothing"
  exit 1
fi

# 1. Parseable YAML. A workflow that does not parse never runs.
for wf in $workflows .github/actions/*/action.yml; do
  [ -e "$wf" ] || continue
  if ! python3 -c 'import sys,yaml; yaml.safe_load(open(sys.argv[1]))' "$wf" 2>/tmp/omni-wf-err; then
    fail "$wf is not valid YAML:"
    sed 's/^/       /' /tmp/omni-wf-err
    status=1
  fi
done

# 2. The incident itself: `secrets.*` inside an `if:` expression.
if hits="$(grep -nE '^[[:space:]]*if:.*secrets\.' $workflows 2>/dev/null)" && [ -n "$hits" ]; then
  fail "the 'secrets' context is not available in 'if:' — this fails the entire run at 0s."
  echo "$hits" | sed 's/^/       /'
  echo "       Fix: expose it via env: on the step, then branch inside run:."
  status=1
fi

# 3. Least privilege: every workflow states its token permissions explicitly.
for wf in $workflows; do
  if ! grep -qE '^permissions:' "$wf"; then
    fail "$wf has no top-level 'permissions:' block (default token perms are broad)"
    status=1
  fi
done

[ $status -eq 0 ] && note "$(echo "$workflows" | wc -w | tr -d ' ') workflow file(s) valid"
exit $status
