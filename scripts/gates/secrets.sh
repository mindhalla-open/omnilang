#!/usr/bin/env bash
# Gate: no credentials in tracked files.
#
# Dependency-free on purpose — it runs identically on a laptop, in GitHub
# Actions and in GitLab CI, with nothing to install and nothing to authenticate.
# It is a floor, not a replacement for provider-side secret scanning.
#
# The repo talks to Anthropic, Ollama and llama.cpp and carries .env examples,
# so the realistic leak is an API key pasted into a doc, an example spec or a
# test fixture.
source "$(dirname "${BASH_SOURCE[0]}")/_lib.sh"

# Patterns are deliberately narrow: a false positive that trains people to
# ignore this gate is worse than a missed exotic key format.
PATTERNS=(
  'sk-ant-[A-Za-z0-9_-]{16,}'          # Anthropic API key
  'sk-[A-Za-z0-9]{32,}'                # OpenAI-style key
  'ghp_[A-Za-z0-9]{30,}'               # GitHub personal access token
  'gho_[A-Za-z0-9]{30,}'               # GitHub OAuth token
  'AKIA[0-9A-Z]{16}'                   # AWS access key id
  '-----BEGIN [A-Z ]*PRIVATE KEY-----' # private key of any flavour
  'xox[baprs]-[A-Za-z0-9-]{10,}'       # Slack token
)

status=0
files="$(git ls-files -- . ':!:*.lock' ':!:*.png' ':!:*.svg' ':!:package-lock.json' ':!:Cargo.lock')"

for pattern in "${PATTERNS[@]}"; do
  # -I skips binaries; the allow-marker lets a doc show a fake key shape on a
  # line that says so out loud.
  if hits="$(printf '%s\n' "$files" | tr '\n' '\0' \
      | xargs -0 grep -InE "$pattern" 2>/dev/null | grep -v 'gate:allow-secret')" \
      && [ -n "$hits" ]; then
    fail "possible credential matching /$pattern/:"
    echo "$hits" | sed 's/^/       /' | head -10
    status=1
  fi
done

# .env files must never be tracked, regardless of content.
if tracked_env="$(git ls-files | grep -E '(^|/)\.env($|\.)' | grep -v '\.env\.example$')" \
    && [ -n "$tracked_env" ]; then
  fail "environment file(s) tracked in git:"
  echo "$tracked_env" | sed 's/^/       /'
  status=1
fi

[ $status -eq 0 ] && note "$(printf '%s\n' "$files" | wc -l | tr -d ' ') tracked file(s) scanned, no credentials found"
exit $status
