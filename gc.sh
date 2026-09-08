#!/usr/bin/env bash
# Commit with a message read from STDIN, never from an inline shell string.
#
# WHY. Inline `git commit -m "..."` puts the message through shell parsing, and prose about code
# reliably contains the two characters that breaks:
#   - a double quote inside a double-quoted string  -> `error: pathspec ... did not match`
#     (hit once today on the phrase "no effect")
#   - a BACKTICK -> command substitution. `git commit -m "the other \`6\`s ..."` ran `6` as a
#     command, printed "bash: 6: command not found", and silently committed a message reading
#     "the other s ...". The commit SUCCEEDED with corrupted text, which is the bad kind of
#     failure: no error status, wrong result.
#
# Both are avoided entirely by never letting the shell see the message.
#
#   ./gc.sh <<'MSG'
#   subject line
#
#   body with "quotes" and `backticks` and $variables, all safe
#   MSG
set -euo pipefail
cd "$(dirname "$0")"
msg=$(mktemp)
trap 'rm -f "$msg"' EXIT
cat > "$msg"
[ -s "$msg" ] || { echo "gc.sh: empty message on stdin"; exit 1; }
git commit -q -F "$msg"
git --no-pager log -1 --format='committed %h %s'
