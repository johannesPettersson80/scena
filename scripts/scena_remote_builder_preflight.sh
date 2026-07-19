#!/usr/bin/env bash
set -euo pipefail

task_slug="${1:-}"
if [[ ! "$task_slug" =~ ^[a-z0-9][a-z0-9-]*$ ]]; then
    echo "usage: scena_remote_builder_preflight.sh <task-slug>" >&2
    exit 2
fi

shared_checkout="${SCENA_SHARED_CHECKOUT:-$HOME/projects/scena}"
isolated_root="${SCENA_ISOLATED_ROOT:-$HOME/.cache/codex-worktrees}"
target_root="${SCENA_TARGET_ROOT:-$HOME/.cache/codex-targets}"
validation_path="$isolated_root/scena-$task_slug"
cargo_target_dir="$target_root/scena-$task_slug"

if [[ -d "$shared_checkout/.git" ]]; then
    shared_checkout_status=present
else
    shared_checkout_status=missing
fi

df -hT "$HOME" "$HOME/.cache" /tmp
du -sh "$cargo_target_dir" "$shared_checkout/target" 2>/dev/null || true
printf 'shared_checkout=%s\n' "$shared_checkout"
printf 'shared_checkout_status=%s\n' "$shared_checkout_status"
printf 'validation_mode=isolated\n'
printf 'validation_path=%s\n' "$validation_path"
printf 'cargo_target_dir=%s\n' "$cargo_target_dir"
