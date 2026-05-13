#!/usr/bin/env bash
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/release-readiness"
LOGS="$OUT/logs"
SUMMARY="$OUT/summary.tsv"
FAILURES=0

mkdir -p "$LOGS"
: > "$SUMMARY"

record_env() {
  cat > "$OUT/env.toml" <<EOF
generated_at_unix = $(date +%s)
SCENA_USE_GPU = "${SCENA_USE_GPU-}"
VK_ICD_FILENAMES = "${VK_ICD_FILENAMES-}"
SCENA_REFERENCE_DIFF = "${SCENA_REFERENCE_DIFF-}"
SCENA_RELEASE_ARTIFACT_ROOT = "${SCENA_RELEASE_ARTIFACT_ROOT-}"
browser_headless = "${BROWSER_HEADLESS-}"
browser_executable = "${BROWSER_EXECUTABLE-}"
EOF
}

run_gate() {
  local name="$1"
  shift
  local log="$LOGS/$name.log"
  printf '==> %s\n' "$name"
  (
    cd "$ROOT"
    "$@"
  ) >"$log" 2>&1
  local status=$?
  printf '%s\t%s\t%s\n' "$name" "$status" "${log#$ROOT/}" >> "$SUMMARY"
  if [[ $status -ne 0 ]]; then
    FAILURES=$((FAILURES + 1))
    printf 'FAIL %s (see %s)\n' "$name" "${log#$ROOT/}"
  fi
  return 0
}

run_optional_gate() {
  local binary="$1"
  local name="$2"
  shift 2
  if command -v "$binary" >/dev/null 2>&1; then
    run_gate "$name" "$@"
  else
    local log="$LOGS/$name.log"
    printf 'MISSING %s\n' "$binary" > "$log"
    printf '%s\t127\t%s\n' "$name" "${log#$ROOT/}" >> "$SUMMARY"
    FAILURES=$((FAILURES + 1))
    printf 'BLOCKED %s: missing %s\n' "$name" "$binary"
  fi
}

record_env

run_gate git-status git status
run_gate git-head git rev-parse HEAD
run_gate rustc rustc -Vv
run_gate cargo-version cargo -V
run_gate rustup-show rustup show
run_gate os uname -a
run_gate cargo-tree-duplicates cargo tree -d
run_gate cargo-update-dry-run cargo update --dry-run --verbose
run_optional_gate vulkaninfo gpu-driver vulkaninfo --summary
run_optional_gate cargo-audit cargo-audit cargo audit
run_optional_gate cargo-deny cargo-deny cargo deny check

run_gate cargo-fmt-check cargo fmt --check
run_gate cargo-clippy-workspace cargo clippy --workspace --all-targets --all-features -- -D warnings
run_gate cargo-test-workspace cargo test --workspace --all-features
run_gate cargo-doc env "RUSTDOCFLAGS=-D warnings" cargo doc --workspace --no-deps --all-features
run_gate cargo-check-examples cargo check --examples --all-features
run_gate cargo-bench-no-run cargo bench --no-run
run_gate cargo-check-wasm cargo check --target wasm32-unknown-unknown --all-features
run_gate cargo-test-xtask cargo test -p xtask

run_gate doctor-full cargo run -p xtask -- doctor --full
run_gate release-readiness cargo run -p xtask -- release-readiness
run_gate package-list cargo package --list

if [[ -z "$(cd "$ROOT" && git status --short)" ]]; then
  run_gate cargo-publish-dry-run cargo publish --dry-run
else
  log="$LOGS/cargo-publish-dry-run.log"
  {
    echo "BLOCKED: current checkout is dirty."
    echo "Public release evidence requires a detached clean worktree at the release commit."
    (cd "$ROOT" && git status --short)
  } > "$log"
  printf '%s\t125\t%s\n' "cargo-publish-dry-run" "${log#$ROOT/}" >> "$SUMMARY"
  FAILURES=$((FAILURES + 1))
  printf 'BLOCKED cargo-publish-dry-run: dirty checkout\n'
fi

run_gate gallery python3 scripts/local_release_gallery.py

printf '\nSummary: %s\n' "${SUMMARY#$ROOT/}"
printf 'Gallery: target/release-readiness/gallery/index.html\n'
if [[ "$FAILURES" -ne 0 ]]; then
  printf 'Status: blocked (%s failed/missing gates)\n' "$FAILURES"
  exit 1
fi
printf 'Status: pass\n'
