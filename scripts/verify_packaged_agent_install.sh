#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_probe_root="$(mktemp -d)"
caller_target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"

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

env CARGO_TARGET_DIR="$caller_target_dir" \
  cargo install \
    --path "$package_dir" \
    --features agent \
    --locked \
    --root "$install_root" \
    --debug \
    --force

version_json="$($install_root/bin/scena --version)"
if ! grep -Eq '"agent"[[:space:]]*:[[:space:]]*true' <<<"$version_json"; then
  echo "packaged agent install gate: installed binary does not report the agent feature" >&2
  echo "$version_json" >&2
  exit 1
fi

echo "packaged agent install gate: passed"
