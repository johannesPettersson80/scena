#!/usr/bin/env bash
set -u

if [ "$#" -lt 2 ]; then
  echo "usage: $0 <lane> <command> [args...]" >&2
  exit 2
fi

lane="$1"
shift

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -d "$ROOT/node_modules/.bin" ]; then
  export PATH="$ROOT/node_modules/.bin:$PATH"
fi

artifact_dir="target/gate-artifacts/release-lanes"
jsonl="${artifact_dir}/${lane}.commands.jsonl"
log_path="${artifact_dir}/${lane}.log"
command_text="$*"

mkdir -p "$artifact_dir"

start_ms="$(python3 -c 'import time; print(int(time.time() * 1000))')"
started_at="$((start_ms / 1000))"

set +e
"$@" > >(tee -a "$log_path") 2> >(tee -a "$log_path" >&2)
exit_code="$?"
set -e

finish_ms="$(python3 -c 'import time; print(int(time.time() * 1000))')"
finished_at="$((finish_ms / 1000))"
duration_ms="$((finish_ms - start_ms))"

if command -v sha256sum >/dev/null 2>&1; then
  log_sha="$(sha256sum "$log_path" | awk '{print $1}')"
else
  log_sha="$(shasum -a 256 "$log_path" | awk '{print $1}')"
fi

if [ "$exit_code" -eq 0 ]; then
  status="passed"
else
  status="failed"
fi

python3 - "$jsonl" "$command_text" "$status" "$duration_ms" "$started_at" "$finished_at" "$log_path" "$log_sha" <<'PY'
import json
import sys

jsonl, command, status, duration_ms, started_at, finished_at, log_path, log_sha = sys.argv[1:]
record = {
    "command": command,
    "status": status,
    "duration_ms": int(duration_ms),
    "duration_source": "ci-wrapper",
    "started_at_unix_seconds": int(started_at),
    "finished_at_unix_seconds": int(finished_at),
    "failure_log_path": log_path,
    "failure_log_sha256": log_sha,
}
with open(jsonl, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(record, sort_keys=True, separators=(",", ":")))
    handle.write("\n")
PY

exit "$exit_code"
