#!/usr/bin/env bash
# LCV-200: run the agent evaluation bench against the live model a settings file names.
# Usage: scripts/agent-bench.sh <settings.json>   (the file must name agent_model)
# The file is LaserCAD's settings JSON (agent_endpoint, agent_api_key, agent_model, ...).
# Writes target/agent-bench/<model>.jsonl plus each task's replies. Never part of the gate.
set -euo pipefail
usage="usage: scripts/agent-bench.sh <settings.json>  (the file must name agent_model)"
if [[ $# -ne 1 || ! -f $1 ]]; then
  echo "$usage" >&2
  exit 2
fi
settings=$(realpath "$1")
cd "$(dirname "$0")/.."
LASERCAD_BENCH_SETTINGS="$settings" cargo test --test it agent_bench_live -- --ignored --nocapture
