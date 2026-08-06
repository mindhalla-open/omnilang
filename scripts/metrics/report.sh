#!/usr/bin/env bash
# Process metrics: lead time, first-pass gate rate, incidents, automation share.
#
# Deliberately NOT measured: story points, velocity, lines changed, PR count.
# Those measure human effort, which is no longer the scarce input, and they grow
# on their own without telling you anything.
#
#   ./scripts/metrics/report.sh        # last 30 days
#   ./scripts/metrics/report.sh 90
#
# Advisory, not a gate — see docs/gates.md ("Known gaps"). Requires `gh`.
set -uo pipefail

DAYS="${1:-30}"
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh CLI not found — install it or read the numbers off the Actions tab." >&2
  exit 1
fi

echo "OmniLang process metrics — last $DAYS days"
echo

# ── Lead time and first-pass gate rate ──────────────────────────────────────
gh pr list --state merged --limit 100 \
  --json number,title,createdAt,mergedAt,headRefName 2>/dev/null \
| DAYS="$DAYS" python3 -c '
import json, os, subprocess, sys
from datetime import datetime, timedelta, timezone

days = int(os.environ["DAYS"])
cutoff = datetime.now(timezone.utc) - timedelta(days=days)

def parse(t):
    return datetime.fromisoformat(t.replace("Z", "+00:00"))

prs = [p for p in json.load(sys.stdin) if p["mergedAt"] and parse(p["mergedAt"]) >= cutoff]

if not prs:
    print("  No PRs merged in the window.")
    sys.exit(0)

hours = sorted((parse(p["mergedAt"]) - parse(p["createdAt"])).total_seconds() / 3600 for p in prs)
median = hours[len(hours) // 2]

print(f"  Lead time (PR opened → merged), n={len(prs)}")
print(f"    median {median:6.1f} h    min {hours[0]:6.1f} h    max {hours[-1]:6.1f} h")
print()

# First-pass gate rate: was the FIRST CI run on the branch green? A branch whose
# first run is red means the spec or the context handed to the implementer was
# not good enough — that is the signal, not developer carelessness.
first_pass = unknown = 0
for p in prs:
    out = subprocess.run(
        ["gh", "run", "list", "--branch", p["headRefName"], "--limit", "50",
         "--json", "conclusion,createdAt"],
        capture_output=True, text=True,
    )
    try:
        runs = sorted(json.loads(out.stdout), key=lambda r: r["createdAt"])
    except Exception:
        runs = []
    if not runs:
        unknown += 1
    elif runs[0]["conclusion"] == "success":
        first_pass += 1

known = len(prs) - unknown
if known:
    print(f"  First-pass gate rate: {first_pass}/{known} = {100*first_pass/known:.0f}%")
    print("    (below 50% sustained → the epic contracts are underspecified, not the tools)")
if unknown:
    print(f"    {unknown} PR(s) had no CI runs at all — that is worse than a red run.")
'

echo
# ── Incidents ────────────────────────────────────────────────────────────────
since="$(python3 -c "
from datetime import datetime, timedelta, timezone
print((datetime.now(timezone.utc) - timedelta(days=$DAYS)).date())")"
fixes="$(git log --since="$since" --format="%s" -- crates runtime/src | grep -c '^fix:')"
echo "  Incident proxy: $fixes 'fix:' commit(s) touching crates/ or runtime/src since $since"
echo "    (a proxy — replace with issues labelled 'incident' once that label is in use)"

echo
# ── Automation share ─────────────────────────────────────────────────────────
gates="$(ls scripts/gates/*.sh 2>/dev/null | grep -vcE '_lib\.sh|run-all\.sh')"
echo "  Machine-enforced conventions: $gates gate(s) in scripts/gates/ + the CI pipeline"
echo "    Registry and known gaps: docs/gates.md"
