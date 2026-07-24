#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if [[ $# -ne 1 ]]; then
  echo "usage: scripts/build_windows_complete_hardware_bundle.sh <output.zip>" >&2
  exit 2
fi

output="$1"
if [[ "$output" != /* ]]; then
  output="$repo_root/$output"
fi
if [[ -e "$output" ]]; then
  echo "refusing to overwrite existing bundle: $output" >&2
  exit 2
fi
mkdir -p "$(dirname "$output")"

source_commit="${SCENA_RELEASE_COMMIT:-$(git rev-parse HEAD)}"
if [[ ! "$source_commit" =~ ^[0-9a-fA-F]{40}$ ]]; then
  echo "SCENA_RELEASE_COMMIT must be an exact 40-hex commit" >&2
  exit 2
fi
source_commit="${source_commit,,}"
if [[ "$source_commit" != "$(git rev-parse HEAD)" ]]; then
  echo "SCENA_RELEASE_COMMIT does not match checkout HEAD" >&2
  exit 2
fi
if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo "Windows release-evidence bundles require a clean committed checkout" >&2
  exit 2
fi

if [[ -z "${CARGO_TARGET_DIR:-}" || "$CARGO_TARGET_DIR" != /* ]]; then
  echo "set CARGO_TARGET_DIR to an absolute task-scoped cache" >&2
  exit 2
fi
case "$CARGO_TARGET_DIR" in
  /|"$HOME"|"$repo_root")
    echo "CARGO_TARGET_DIR is too broad for task-scoped proof builds: $CARGO_TARGET_DIR" >&2
    exit 2
    ;;
esac

wasm_pack_version="$(wasm-pack --version)"
if [[ "$wasm_pack_version" != "wasm-pack 0.14.0" ]]; then
  echo "expected wasm-pack 0.14.0, got $wasm_pack_version" >&2
  exit 2
fi

work_root="$(mktemp -d "${TMPDIR:-/tmp}/scena-windows-complete-bundle.XXXXXX")"
trap 'rm -rf "$work_root"' EXIT
bundle_root="$work_root/bundle"
mkdir -p \
  "$bundle_root/bin" \
  "$bundle_root/target/m6-browser-pkg" \
  "$bundle_root/target/pf01-output-toggle-browser-pkg" \
  "$bundle_root/tests/browser" \
  "$bundle_root/tests/release" \
  "$bundle_root/tests/assets/gltf" \
  "$bundle_root/tests/visual/references" \
  "$bundle_root/scripts" \
  "$bundle_root/src/browser_probe" \
  "$bundle_root/.codex"

SCENA_RELEASE_COMMIT="$source_commit" wasm-pack build --dev --target web \
  --out-dir "$bundle_root/target/m6-browser-pkg" . --features browser-probe
SCENA_RELEASE_COMMIT="$source_commit" wasm-pack build --dev --target web \
  --out-dir "$bundle_root/target/pf01-output-toggle-browser-pkg" . --features scene-host

windows_target="x86_64-pc-windows-gnu"
profile_dir="$CARGO_TARGET_DIR/$windows_target/perf-test"
deps_dir="$profile_dir/deps"
cargo build --profile perf-test --target "$windows_target" \
  --example native_surface_hardware_proof
cp "$profile_dir/examples/native_surface_hardware_proof.exe" \
  "$bundle_root/bin/scena-native-hardware-proof.exe"

build_test_executable() {
  local test_name="$1"
  local output_name="$2"
  shift 2
  mkdir -p "$deps_dir"
  rm -f "$deps_dir/${test_name}-"*.exe
  cargo test --profile perf-test --target "$windows_target" "$@" \
    --test "$test_name" --no-run
  local matches=("$deps_dir/${test_name}-"*.exe)
  if [[ ${#matches[@]} -ne 1 || ! -f "${matches[0]}" ]]; then
    echo "expected exactly one $test_name Windows test executable, found ${#matches[@]}" >&2
    exit 1
  fi
  cp "${matches[0]}" "$bundle_root/bin/$output_name"
}

build_test_executable fr06_semantic_aov scena-fr06-native-hardware-proof.exe \
  --features scene-host
build_test_executable c09_gpu_resource_lifecycle scena-q04-gpu-resource-lifecycle.exe
build_test_executable p01_shader_module_cache scena-p01-shader-module-cache.exe
build_test_executable m8_real_asset_proof scena-m8-waterbottle-full-frame.exe
build_test_executable q07_antialiasing_effect scena-q07-antialiasing-effect.exe
build_test_executable q01_waterbottle_cpu_reference scena-q11-reference-stability.exe
build_test_executable transmission_parity scena-q08-transmission-parity.exe
build_test_executable c13_depth_clipping_parity scena-q08-clipping-parity.exe
build_test_executable dynamic_transform_parity scena-q08-dynamic-parity.exe
build_test_executable pbr_brdf_parity scena-q08-pbr-parity.exe
build_test_executable pf08_texture_bake_parity scena-q08-texture-bake-parity.exe

cp -R tests/browser/. "$bundle_root/tests/browser/"
cp -R tests/release/. "$bundle_root/tests/release/"
cp -R tests/assets/gltf/. "$bundle_root/tests/assets/gltf/"
cp -R tests/visual/references/. "$bundle_root/tests/visual/references/"
cp tests/q07_antialiasing_effect.rs "$bundle_root/tests/"
cp \
  tests/transmission_parity.rs \
  tests/c13_depth_clipping_parity.rs \
  tests/dynamic_transform_parity.rs \
  tests/pbr_brdf_parity.rs \
  tests/pf08_texture_bake_parity.rs \
  "$bundle_root/tests/"
cp scripts/round_e_material_evaluator.cjs "$bundle_root/scripts/"
cp src/browser_probe.rs "$bundle_root/src/"
cp src/browser_probe/parity.rs "$bundle_root/src/browser_probe/"
cp package.json package-lock.json Cargo.lock AGENTS.md "$bundle_root/"
cp -R .codex/skills "$bundle_root/.codex/"
cp scripts/run_windows_complete_hardware_proof.ps1 "$bundle_root/run-proof.ps1"
printf '%s\n' "$source_commit" > "$bundle_root/source-commit.txt"

(
  cd "$bundle_root"
  while IFS= read -r relative; do
    sha256sum "$relative"
  done < <(find . -type f ! -name bundle-files.sha256 -printf '%P\n' | LC_ALL=C sort) \
    > bundle-files.sha256
)

(
  cd "$bundle_root"
  zip -q -r "$output" .
)
sha256sum "$output"
printf 'source_commit=%s\n' "$source_commit"
printf 'bundle=%s\n' "$output"
