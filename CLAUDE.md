# scena project notes

Project-local notes for Claude and contributors. This file is loaded into every
Claude Code session and should stay terse; durable, machine-derivable facts
(commit history, current code structure) live in the source. Things that
**only** live here are stuff a fresh reader cannot derive by looking at code:
test-rig environment flags, lavapipe/Vulkan quirks, where artifacts go, and
why certain non-obvious choices were made.

## Test environment flags

Tests in `tests/m8_real_asset_proof.rs` and a few neighbouring suites read
runtime environment flags to switch render path or vulkan driver. They are
listed here so reviewers don't get caught by silent fallbacks:

| Flag | What it controls | Default when unset |
|---|---|---|
| `SCENA_USE_GPU` | When set, m8 real-asset test goes through `Renderer::headless_gpu` (the headline PBR path). Otherwise the CPU rasterizer renders. | unset → CPU rasterizer |
| `VK_ICD_FILENAMES` | Vulkan loader picks which ICD driver to use. On the Pi 5 / V3DV-broken hosts, point this at `/usr/share/vulkan/icd.d/lvp_icd.json` to force Mesa lavapipe (software Vulkan). | system default |
| `SCENA_REFERENCE_DIFF` | When set, m8 real-asset test runs an additional reference-image ΔE diff against `tests/assets/gltf/khronos/WaterBottle/reference_512.png`. | unset → diff skipped (asserts still run) |

To exercise the headline render on the Pi 5:

```
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json SCENA_USE_GPU=1 \
  cargo test --test m8_real_asset_proof
```

When `SCENA_USE_GPU` is set but the system has no working GPU adapter, the
test logs the fallback to stderr and continues with the CPU rasterizer — it
does not silently pass as if GPU coverage had run.

## Gate artifact locations

Render and capability artifacts land under `target/gate-artifacts/`. Tests
that emit artifacts always print the path to stderr so the human can open
them.

Notable locations:

- `target/gate-artifacts/m8-real-asset/waterbottle.png` — WaterBottle proof
  output (CPU or GPU depending on `SCENA_USE_GPU`).
- `target/gate-artifacts/m8-real-asset/waterbottle_renderer.toml` — companion
  metadata: which renderer ran, GPU adapter name (if any), test SHA-256 of
  the bundled reference.

## Doctor

`cargo run -p xtask -- doctor --full` is the source of truth for "is the
codebase in a shippable shape". Failing doctor blocks release-readiness.

Doctor's truth substrings pin contract text in specific files. When the
underlying file is rewritten (Stage C2 moved glTF parsing from a hand-rolled
walker to the `gltf` crate's typed accessors), doctor's pinned strings need
updating in lockstep.
