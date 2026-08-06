#!/usr/bin/env bash
# Gate: every diagnostic code emitted by the analyzer is documented, and every
# documented code still exists in the analyzer.
#
# docs/18-error-codes.md claims to be "generated from the analyzer source", but
# nothing enforced it: E0242 (unsupported service target) shipped undocumented.
# Codes are a public contract — users grep for them and filter on them — so
# drift here is a compatibility bug, not a docs nit.
source "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

DOC="docs/18-error-codes.md"
status=0

in_source="$(grep -rhoE '"E[0-9]{4}"' crates/*/src | tr -d '"' | sort -u)"
in_docs="$(grep -ohE '\bE[0-9]{4}\b' "$DOC" | sort -u)"

if [ -z "$in_source" ]; then
  fail "found no diagnostic codes in crates/*/src — the gate is checking nothing"
  exit 1
fi

undocumented="$(comm -23 <(echo "$in_source") <(echo "$in_docs"))"
if [ -n "$undocumented" ]; then
  fail "emitted but not documented in $DOC:"
  for code in $undocumented; do
    site="$(grep -rn "\"$code\"" crates/*/src | head -1)"
    echo "       $code  ← $site"
  done
  status=1
fi

stale="$(comm -13 <(echo "$in_source") <(echo "$in_docs"))"
if [ -n "$stale" ]; then
  fail "documented in $DOC but no longer emitted (removing a code is a breaking change — see docs/19-versioning-policy.md):"
  echo "$stale" | sed 's/^/       /'
  status=1
fi

[ $status -eq 0 ] && note "$(echo "$in_source" | wc -l | tr -d ' ') diagnostic codes, source and docs agree"
exit $status
