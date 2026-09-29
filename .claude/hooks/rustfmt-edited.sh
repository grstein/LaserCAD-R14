#!/usr/bin/env bash
# PostToolUse(Edit|Write): format the edited Rust file in place.
f=$(jq -r '.tool_input.file_path // empty')
[[ $f == *.rs && -f $f ]] && rustfmt "$f" 2>/dev/null
exit 0
