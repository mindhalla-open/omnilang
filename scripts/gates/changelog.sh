#!/usr/bin/env bash
# Gate: behaviour changes are recorded in CHANGELOG.md.
#
# Replaces the PR checkbox "I have updated the CHANGELOG" — a promise a
# reviewer had to verify by hand and an agent never saw at all.
#
# Escape hatch: put [no-changelog] in the branch's latest commit message when a
# change genuinely has no user-visible effect. Deliberate and visible in history,
# which is the difference between an exception and a hole.
source "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

WATCHED='^(crates/|runtime/src/|action\.yml$)'

base="${BASE_REF:-}"
if [ -z "$base" ]; then
  for candidate in origin/main main; do
    if git rev-parse --verify --quiet "$candidate" >/dev/null; then base="$candidate"; break; fi
  done
fi

if [ -z "$base" ]; then
  note "SKIPPED — no base ref (set BASE_REF=<sha>); nothing was checked"
  exit 0
fi

merge_base="$(git merge-base "$base" HEAD 2>/dev/null)" || merge_base="$base"
# Committed changes on the branch, plus anything still in the working tree — a
# local run that ignores uncommitted work would report green on the very change
# you are about to push.
changed="$(printf '%s\n%s\n' \
  "$(git diff --name-only "$merge_base"...HEAD)" \
  "$(git status --porcelain --untracked-files=all | cut -c4-)" \
  | sed '/^$/d' | sort -u)"

if [ -z "$changed" ]; then
  note "no changes vs $base"
  exit 0
fi

if ! printf '%s\n' "$changed" | grep -qE "$WATCHED"; then
  note "no changes under crates/ or runtime/src — changelog not required"
  exit 0
fi

if printf '%s\n' "$changed" | grep -q '^CHANGELOG\.md$'; then
  note "code changed and CHANGELOG.md was updated"
  exit 0
fi

if git log -1 --pretty=%B HEAD | grep -q '\[no-changelog\]'; then
  note "code changed; waived by [no-changelog] in the latest commit message"
  exit 0
fi

fail "code changed under crates/ or runtime/src but CHANGELOG.md was not updated"
printf '%s\n' "$changed" | grep -E "$WATCHED" | sed 's/^/       /' | head -20
exit 1
