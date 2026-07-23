#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]] || [[ ! "$1" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$ ]]; then
  echo "usage: scripts/scena_task_cache_status.sh <task-slug>" >&2
  echo "task slug must be 1-64 ASCII letters, digits, dot, underscore, or hyphen and cannot start with punctuation" >&2
  exit 2
fi

task_slug=$1
cache_root=${SCENA_TASK_CACHE_ROOT:-"${HOME}/.cache"}
validation_path="${cache_root}/codex-worktrees/scena-${task_slug}"
cargo_target_dir="${cache_root}/codex-targets/scena-${task_slug}"
task_tmpdir="${cargo_target_dir}/tmp"

python3 - "$task_slug" "$validation_path" "$cargo_target_dir" "$task_tmpdir" <<'PY'
import json
import os
import pathlib
import sys
import time

task_slug = sys.argv[1]
now = int(time.time())
entries = []
for kind, raw_path in zip(
    ("validation_checkout", "cargo_target", "temp"),
    sys.argv[2:],
):
    path = pathlib.Path(raw_path).absolute()
    exists = path.exists()
    size = 0
    modified = 0
    if exists:
        try:
            modified = int(path.stat().st_mtime)
        except OSError:
            modified = 0
        if path.is_file() or path.is_symlink():
            try:
                size = path.lstat().st_size
            except OSError:
                size = 0
        else:
            for root, directories, files in os.walk(path, followlinks=False):
                for name in directories + files:
                    candidate = pathlib.Path(root, name)
                    try:
                        stat = candidate.lstat()
                    except OSError:
                        continue
                    size += stat.st_size
                    modified = max(modified, int(stat.st_mtime))
    entries.append(
        {
            "kind": kind,
            "path": str(path),
            "exists": exists,
            "size_bytes": size,
            "modified_unix_seconds": modified,
            "age_seconds": max(0, now - modified) if modified else None,
            "reproducible": True,
            "retention": "retain while the task or its evidence is active",
            "cleanup_authority": "explicit_operator_request_for_this_exact_path_only",
        }
    )

print(
    json.dumps(
        {
            "schema": "scena.task_cache_status.v1",
            "task_slug": task_slug,
            "read_only": True,
            "generated_unix_seconds": now,
            "entries": entries,
        },
        indent=2,
        sort_keys=True,
    )
)
PY
