#!/usr/bin/env bash
# Clean-tree publish dry-run helper for Phase 8 closure.
#
# Detaches a fresh git worktree at the requested commit (default HEAD) so
# `cargo publish --dry-run` runs against an unmodified tree free of
# node_modules, target/, and other working-tree artifacts. Records the log
# and per-step exit codes under
# `target/gate-artifacts/release-lanes/publish-dry-run.log` so the maintainer
# can attach the evidence to the v1.0 release-readiness bundle without
# re-running ad-hoc commands.
#
# Closes the publish-dry-run row in
# `docs/decisions/ADR-0006-Local-Release-Candidate-Closure.md` once a
# successful run records `status=passed` for every gate step.
#
# Usage:
#   scripts/release_publish_dry_run.sh                # use HEAD
#   scripts/release_publish_dry_run.sh <commit-sha>   # use the named commit
#
# The helper does NOT push, tag, or publish — it only proves the clean-tree
# gates would pass. Phase 8 still requires the maintainer to run the actual
# `cargo publish` command after `cargo publish --dry-run` succeeds.

set -euo pipefail

target_commit="${1:-HEAD}"
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

resolved_commit="$(git rev-parse "$target_commit")"
short_commit="$(git rev-parse --short "$resolved_commit")"
worktree_dir="/tmp/scena-publish-dry-run-${short_commit}"
artifact_dir="${repo_root}/target/gate-artifacts/release-lanes"
log_path="${artifact_dir}/publish-dry-run.log"

mkdir -p "$artifact_dir"
: > "$log_path"

# Cleanup any prior worktree at the same path.
if [ -d "$worktree_dir" ]; then
  git worktree remove --force "$worktree_dir" 2>/dev/null || rm -rf "$worktree_dir"
fi

git worktree add --detach "$worktree_dir" "$resolved_commit" 2>&1 \
  | tee -a "$log_path"

cleanup() {
  cd "$repo_root"
  git worktree remove --force "$worktree_dir" 2>/dev/null || rm -rf "$worktree_dir"
}
trap cleanup EXIT

cd "$worktree_dir"

declare -i overall_status=0

run_step() {
  local label="$1"
  shift
  echo >> "$log_path"
  echo "==== ${label} ====" | tee -a "$log_path"
  echo "$ $*" | tee -a "$log_path"
  local start_ms
  start_ms="$(date +%s%3N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1000))')"
  set +e
  "$@" > >(tee -a "$log_path") 2> >(tee -a "$log_path" >&2)
  local code="$?"
  set -e
  local finish_ms
  finish_ms="$(date +%s%3N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1000))')"
  local duration=$((finish_ms - start_ms))
  echo "[${label}] exit_code=${code} duration_ms=${duration}" | tee -a "$log_path"
  if [ "$code" -ne 0 ]; then
    overall_status=1
  fi
  return 0
}

echo "==== publish-dry-run @ ${resolved_commit} ====" | tee -a "$log_path"
echo "worktree: ${worktree_dir}" | tee -a "$log_path"
echo "artifact: ${log_path}" | tee -a "$log_path"

run_step "cargo fmt --check"          cargo fmt --check
run_step "cargo clippy"               cargo clippy --all-targets -- -D warnings
run_step "cargo test"                 cargo test
run_step "cargo check --examples"     cargo check --examples
RUSTDOCFLAGS="-D warnings" run_step "cargo doc"  cargo doc --no-deps --all-features
run_step "cargo doctor --full"        cargo run -p xtask -- doctor --full
run_step "cargo claim-audit"          cargo run -p xtask -- claim-audit
run_step "cargo release-readiness"    cargo run -p xtask -- release-readiness
run_step "cargo publish --dry-run"    cargo publish --dry-run

echo >> "$log_path"
if [ "$overall_status" -eq 0 ]; then
  echo "==== publish-dry-run status=passed commit=${resolved_commit} ====" | tee -a "$log_path"
else
  echo "==== publish-dry-run status=failed commit=${resolved_commit} ====" | tee -a "$log_path"
fi

exit "$overall_status"
