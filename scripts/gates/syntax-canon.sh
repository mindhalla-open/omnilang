#!/usr/bin/env bash
# Gate: the canonical syntax is brace-free (ADR-0009, package.syntax_version 0.2).
#
# The parser still accepts `{}` blocks for backward compatibility, so nothing in
# the compiler stops a brace block from creeping back into an example or a doc
# snippet. Two occurrences already survived the 2026-06 conversion; a reviewer's
# eye does not scale to 82 fenced snippets, so this does it instead.
#
# What counts as a violation: a declaration head that opens a `{` block.
# What does not: `type X = { ... }` — an inline structural type literal, which
# has no brace-free form and is not a block.
source "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

KEYWORDS='module|import|export|type|service|component|pipeline|workflow|agent|schema|policy|constraint|mixin|entity|action|rule|operation|test|scenario|enum|struct|infra|deployment'
BLOCK_OPENER="^[[:space:]]*(${KEYWORDS})[[:space:]][^=]*\{[[:space:]]*$"

status=0

# 1. Examples are normative — they are the corpus users copy from.
if hits="$(grep -rnE "$BLOCK_OPENER" examples/ 2>/dev/null)" && [ -n "$hits" ]; then
  fail "brace-style block(s) in examples/ — canon is brace-free:"
  echo "$hits" | sed 's/^/       /'
  status=1
fi

# 2. Fenced snippets in docs. Extract ```omnilang blocks, keep the real line
#    numbers so a failure points at something clickable.
extract_and_check() {
  local file="$1"
  awk -v file="$file" -v re="$BLOCK_OPENER" '
    /^```omnilang[[:space:]]*$/ { inblock = 1; next }
    /^```/                      { inblock = 0; next }
    inblock && $0 ~ re          { printf "%s:%d:%s\n", file, NR, $0 }
  ' "$file"
}

for doc in docs/*.md README.md; do
  [ -e "$doc" ] || continue
  if hits="$(extract_and_check "$doc")" && [ -n "$hits" ]; then
    fail "brace-style block(s) in $doc:"
    echo "$hits" | sed 's/^/       /'
    status=1
  fi
done

# 3. Fence tag must be `omnilang`, otherwise check 2 silently skips the snippet
#    and this gate quietly stops gating.
if hits="$(grep -rnE '^```omni[[:space:]]*$' docs/*.md README.md 2>/dev/null)" && [ -n "$hits" ]; then
  fail "use \`\`\`omnilang (not \`\`\`omni) so snippets stay machine-extractable:"
  echo "$hits" | sed 's/^/       /'
  status=1
fi

[ $status -eq 0 ] && note "no brace-style blocks in examples/ or docs/"
exit $status
