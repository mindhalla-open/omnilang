"""Compare measured build-cost estimates against scripts/gates/budgets.json.

Reads "<spec-path> <usd>" lines on stdin. Invoked by budgets.sh.
"""

import json
import sys

BUDGETS = sys.argv[1]

budgets = json.load(open(BUDGETS))
caps = {k: v for k, v in budgets["examples"].items()}
total_cap = budgets["total_usd"]

actual = {}
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    path, usd = line.rsplit(" ", 1)
    actual[path] = float(usd)

if not actual:
    print("   ✗ measured no specs — the gate is checking nothing", file=sys.stderr)
    sys.exit(1)

failures = []
for path, usd in sorted(actual.items()):
    if path not in caps:
        failures.append(
            f"{path}: costs ${usd:.4f} and has no entry in {BUDGETS} — add one deliberately"
        )
    elif usd > caps[path]:
        failures.append(f"{path}: ${usd:.4f} exceeds cap ${caps[path]:.4f}")

for path in sorted(set(caps) - set(actual)):
    failures.append(f"{path}: budgeted but the spec no longer exists — drop the entry")

total = sum(actual.values())
if total > total_cap:
    failures.append(f"total ${total:.4f} exceeds ${total_cap:.4f}")

for f in failures:
    print(f"   ✗ {f}", file=sys.stderr)
if failures:
    sys.exit(1)

print(f"   · {len(actual)} spec(s), ${total:.4f} of ${total_cap:.4f} budget")
