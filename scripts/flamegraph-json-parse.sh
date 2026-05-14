#!/usr/bin/env bash
# Run cargo flamegraph for the profile_json_parse binary (bounded wall time; same fixtures as the
# json_parse Criterion bench). On WSL, kernel-specific linux-tools-* packages
# are often missing; Ubuntu's generic perf may still work — this script picks one if PERF is unset.
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -z "${PERF:-}" ]]; then
  for candidate in /usr/lib/linux-tools/*/perf; do
    if [[ -x "$candidate" ]]; then
      export PERF="$candidate"
      echo "Using perf: $PERF" >&2
      break
    fi
  done
fi

if [[ -z "${PERF:-}" ]] && ! command -v perf >/dev/null 2>&1; then
  echo "perf not found. Try:" >&2
  echo "  sudo apt install linux-tools-common linux-tools-generic" >&2
  echo "Then re-run this script, or set PERF=/usr/lib/linux-tools/<ver>-generic/perf" >&2
  echo "If generic perf still fails on WSL, build perf from the WSL2 kernel tree (tools/perf)." >&2
  exit 1
fi

exec cargo flamegraph --profile bench --bin profile_json_parse "$@"
