#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  echo "Q11 reference candidates require a clean checkout" >&2
  exit 1
fi

candidate_id="$(date -u +%Y%m%dT%H%M%SZ)-$(git rev-parse --short=12 HEAD)"
candidate_dir="target/reference-candidates/q01-waterbottle-$candidate_id"
if [[ -e "$candidate_dir" ]]; then
  echo "candidate directory already exists: $candidate_dir" >&2
  exit 1
fi

export SCENA_RELEASE_COMMIT="$(git rev-parse HEAD)"
export SCENA_Q11_REFERENCE_CANDIDATE_DIR="$candidate_dir"
cargo test --test q01_waterbottle_cpu_reference \
  q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact

echo "candidate staged at $candidate_dir"
echo "review candidate.json, candidate.png, diff-heatmap.png, and the external Blender anchor"
echo "promotion requires a separately authored approval JSON and scripts/promote_q01_waterbottle_reference.cjs"
