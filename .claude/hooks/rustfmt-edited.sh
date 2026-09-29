#!/usr/bin/env bash
# PostToolUse(Edit|Write): format the edited Rust file in place.
# Silent on success. On failure, exit 2 so Claude Code shows stderr to the agent.
f=$(jq -r '.tool_input.file_path // empty')
[[ $f == *.rs && -f $f ]] || exit 0
if ! err=$(rustfmt "$f" 2>&1 >/dev/null); then
  printf 'rustfmt failed on %s:\n%s\n' "$f" "$err" >&2
  exit 2
fi
exit 0
