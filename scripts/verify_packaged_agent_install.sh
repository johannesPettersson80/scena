#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_probe_root="$(mktemp -d)"
caller_target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
package_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$repo_root/Cargo.toml" | head -n 1)"
if [[ -z "$package_version" ]]; then
  echo "packaged agent install gate: package version is unavailable" >&2
  exit 1
fi
package_version_regex="${package_version//./\\.}"

cleanup() {
  find "$package_probe_root" -depth -delete
}
trap cleanup EXIT

package_target_dir="$package_probe_root/package-target"
install_root="$package_probe_root/install"
mkdir -p "$package_target_dir" "$install_root"

(
  cd "$repo_root"
  env CARGO_TARGET_DIR="$package_target_dir" \
    cargo package --locked --allow-dirty --no-verify
)

package_archive="$(find "$package_target_dir/package" -maxdepth 1 -type f -name 'scena-*.crate' -print -quit)"
if [[ -z "$package_archive" ]]; then
  echo "packaged agent install gate: cargo package did not produce a scena archive" >&2
  exit 1
fi

unpacked_root="$package_probe_root/unpacked"
mkdir -p "$unpacked_root"
tar -xzf "$package_archive" -C "$unpacked_root"
package_dir="$(find "$unpacked_root" -mindepth 1 -maxdepth 1 -type d -name 'scena-*' -print -quit)"
if [[ -z "$package_dir" ]]; then
  echo "packaged agent install gate: scena archive did not contain a package directory" >&2
  exit 1
fi

required_packaged_assets=(
  "tests/assets/photo/final/photo_final_policy_v1.json"
  "tests/assets/environment/neutral-studio.fixture.txt"
  "tests/assets/environment/generated/studio_small_03_128x64.hdr"
  "tests/assets/environment/polyhaven/studio_small_08_1k.hdr"
  "tests/assets/gltf/material_variants_scene.gltf"
  "tests/assets/gltf/animated_triangle_scene.glb"
  "tests/assets/gltf/cad_plate_drawing_scene.gltf"
  "docs/guides/llm-app-builder.md"
)
for asset in "${required_packaged_assets[@]}"; do
  if [[ ! -f "$package_dir/$asset" ]]; then
    echo "packaged agent install gate: archive omitted compile-time asset $asset" >&2
    exit 1
  fi
done

# The public CLI has a default discovery/validation profile and the documented
# `agent` application-builder profile. Compile the former from the extracted
# archive before installing and executing the latter.
env CARGO_TARGET_DIR="$caller_target_dir" \
  cargo check \
    --manifest-path "$package_dir/Cargo.toml" \
    --bin scena \
    --locked

env CARGO_TARGET_DIR="$caller_target_dir" \
  cargo install \
    --path "$package_dir" \
    --features agent \
    --locked \
    --root "$install_root" \
    --debug \
    --force

version_json="$($install_root/bin/scena --version)"
required_version_fields=(
  "\"package_version\"[[:space:]]*:[[:space:]]*\"$package_version_regex\""
  '"agent"[[:space:]]*:[[:space:]]*true'
  '"scene_host"[[:space:]]*:[[:space:]]*true'
  '"inspection"[[:space:]]*:[[:space:]]*true'
)
for field in "${required_version_fields[@]}"; do
  if ! grep -Eq "$field" <<<"$version_json"; then
    echo "packaged agent install gate: installed binary omitted required version field $field" >&2
    echo "$version_json" >&2
    exit 1
  fi
done

echo "packaged agent install gate: passed"
