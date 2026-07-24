# scena project notes

Project-local notes for Claude and contributors. This file is loaded into every
Claude Code session and should stay terse; durable, machine-derivable facts
(commit history, current code structure) live in the source. Things that
**only** live here are stuff a fresh reader cannot derive by looking at code:
test-rig environment flags, lavapipe/Vulkan quirks, where artifacts go, and
why certain non-obvious choices were made.

## Test environment flags

Tests and browser/build scripts read the following environment flags. Required
release lanes must set only the flags named by their workflow; a local override
does not turn diagnostic output into release evidence.

| Flag | What it controls | Default when unset |
|---|---|---|
| `SCENA_A03_BIN` | Exact prebuilt release-profile `scena` binary used by the clean-directory canonical-guide smoke. | test builds its own binary only in the final packaged smoke |
| `SCENA_A04_BIN` | Exact installed/packaged CLI binary used by the install-contract smoke. | test builds its own binary only in the final packaged smoke |
| `SCENA_A04_EXPECT_AGENT` | Declares whether the A04 packaged binary was built with the `agent` feature so the smoke can enforce the matching command surface. | unset → core/default install contract |
| `SCENA_A05_BIN` | Exact installed/packaged CLI binary used to prove public agent-guide discovery outside the repository. | test builds its own binary only in the final packaged smoke |
| `SCENA_USE_GPU` | Legacy test/proof metadata bit written into the WaterBottle renderer companion. The `scena` CLI deliberately ignores it; use the explicit `--gpu` flag to select GPU execution. | unset → metadata records false |
| `VK_ICD_FILENAMES` | Vulkan loader picks which ICD driver to use. On the Pi 5 / V3DV-broken hosts, point this at `/usr/share/vulkan/icd.d/lvp_icd.json` to force Mesa lavapipe (software Vulkan). | system default |
| `SCENA_REFERENCE_DIFF` | Enables the WaterBottle GPU full-frame release oracle: at least 95% of pixels must be within RGB Chebyshev distance 16 of `reference_512.png`, and a horizontal-mirror mutation must fail. Required CI/release and exact-candidate Windows hardware lanes set it. | unset → diagnostic-only result with `release_evidence:false`; required release lanes use `1` |
| `SCENA_Q11_REFERENCE_CANDIDATE_DIR` | Writes a candidate-only Q11 reference frame and metadata beneath the exact requested directory; it never replaces an approved reference. | unset → verify the committed reference without staging a candidate |
| `SCENA_REQUIRE_AA_EFFECT_PROOF` | Requires the native high-contrast diagonal None/FXAA/MSAA pixel-effect proof on a physical GPU; no-op AA and whole-frame blur mutations must fail. | unset → focused synthetic oracle only; required Metal/Windows hardware lanes use `1` |
| `SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS` | When set, adapter-sensitive local headless-GPU visual assertions run instead of writing fail-closed `release_evidence=false` metadata. Use only on an approved visual-proof lane. | unset → fail-closed metadata, no release claim |
| `SCENA_RUN_EXPENSIVE_CPU_RELEASE_TESTS` | When set, the long WaterBottle CPU release proof runs and writes the CPU PNG artifact. Use only on an approved CPU release-proof lane. | unset → fail-closed metadata, no release claim |
| `SCENA_RUN_DEDICATED_4K_BENCHMARK` | When set, the M9 dedicated 4K benchmark writes measured `m9-benchmarks-4k.json`; otherwise the normal suite writes a required-lane artifact with `release_evidence=false`. | unset → fail-closed requirement metadata |
| `SCENA_RUN_M9_PLATFORM_BENCHMARK` | Enables the exact lane-specific M9 baseline measurement. CI and release workflows run it separately with one test thread so the broad functional suite cannot distort timing evidence. | unset → fail-closed requirement metadata only |
| `SCENA_M9_TIMING_POLICY` | Selects `strict-controlled` wall-clock enforcement or `report-only-hosted` timing observation. Sample validity and allocation budgets stay blocking in both modes; GitHub-hosted workflows must use report-only because their VM performance is uncontrolled. | unset → `strict-controlled` |
| `SCENA_RUN_PF00_BENCHMARK` | Enables the dedicated optimized PF00 representative-workload harness. Until all ten registered workloads are measured, its aggregate artifact remains `release_evidence=false`. | unset → fail-closed requirement metadata only |
| `SCENA_REAGGREGATE_PF00` | Rebuilds the PF00 aggregate from the existing ten immutable workload artifacts after validating their schemas, identities, timestamps, checksums, and 100-sample distributions. It does not rewrite raw workload provenance. | unset → reaggregation test skips without changing artifacts |
| `SCENA_RUN_PF03_STORAGE_BENCHMARK` | Enables the optimized 100,002-vertex prepared-storage producer for shared model buffers, draw transforms, allocation bytes, and copy-byte accounting. | unset → focused producer skipped |
| `SCENA_RUN_PF10_OCCLUSION_BENCHMARK` | Enables the optimized PF10 CPU occlusion on/off distribution producer for below-threshold, dense-overlap, and sparse scenes. | unset → fail-closed requirement metadata only |
| `SCENA_RUN_CONTROLLED_P01_BENCHMARK` | Enables the cold-device versus warm-device-cache full-prepare distribution for triangle shader reuse. Physical-GPU p95 acceptance is required for a release performance claim; software adapters report inconclusive evidence. | unset → deterministic cache-count and pixel-parity proof only |
| `SCENA_BENCHMARK_PROFILE` | Records the exact Cargo profile for M9/PF00 performance evidence. Required optimized lanes set this to `perf-test`; an optimized build without the variable is labeled `optimized-unspecified`. | debug assertions → `unoptimized-test`; otherwise `optimized-unspecified` |
| `SCENA_BENCHMARK_COMMAND` | Records the exact benchmark command in M9/PF00 provenance. Required performance workflows set the literal command they execute. | explicit unavailable reason; no release performance claim |
| `SCENA_BENCHMARK_CPU` | Overrides CPU-model discovery when `/proc/cpuinfo` is unavailable or the lane needs a canonical hardware label. | `/proc/cpuinfo` model name, then architecture with unknown-model suffix |
| `SCENA_RELEASE_COMMIT` | Overrides the commit stamped into locally generated release evidence. CI should normally use `GITHUB_SHA`; this override is for exact-commit local or isolated-builder proof generation. | unset → `GITHUB_SHA`, then fail-closed local provenance |
| `SCENA_RELEASE_PROFILE` | Records the Cargo profile label in generated M5 release artifacts. Required workflows set it explicitly for the provenance-producing command. | debug assertions → `test-unoptimized`; otherwise `test-optimized` |
| `SCENA_BUILD_HEARTBEAT_MS` | Milliseconds between progress messages from `scripts/build_demo_wasm.js`. | `20000` |
| `CHROMIUM` | Browser executable override used by showcase/reference/browser probes. | Playwright-managed browser |
| `SCENA_ROUND_E_REFERENCE_SHOWCASE` | Optional output path for a Round E reference-generation screenshot. | no extra screenshot |
| `SCENA_MATERIAL_PROOF_URL` | URL override for the deployed material-presets probe. | canonical public proof URL |
| `SCENA_SHOWCASE_CONNECTOR_ONLY` | Restricts the local showcase probe to connector behavior when set to `1`. | full showcase probe |
| `SCENA_SHOWCASE_SECTION_BUDGET_MS` | Overrides the showcase section-activation latency budget in milliseconds. | script-defined budget |
| `SCENA_BROWSER_BACKENDS` | Comma-separated browser backends selected by browser proof scripts. Required workflows set one explicit backend. | script-defined backend set |
| `SCENA_WEBGPU_BROWSER` | Test-only browser-engine selector for WebGPU hardware proofs. Supported values are `chromium` and `firefox`; required workflows normally use the default. | `chromium` |
| `SCENA_WEBGL2_BROWSER` | Test-only browser-engine selector for WebGL2 hardware proofs. Supported values are `chromium` and `firefox`; required workflows normally use the default. | `chromium` |
| `SCENA_BROWSER_WORKFLOWS` | Comma-separated subset of M6 browser workflows; unknown entries fail. | all registered workflows |
| `SCENA_BROWSER_EXECUTABLE` | M6/scene-host browser executable override; takes precedence over `CHROMIUM`. | Playwright-managed browser |
| `SCENA_BROWSER_ALLOW_UNAVAILABLE` | Diagnostic-only M6 switch allowing a specifically classified unavailable WebGPU result. It is forbidden in required CI/release jobs. | strict failure |
| `SCENA_REQUIRE_PARITY` | Required GPU-lane mode. WebGPU adapter absence, software adapters, missing renderer-owned readback, zero GPU work, and native CPU fallback fail the lane directly; only explicitly named diagnostic/headless lanes omit it. | unset -> diagnostic fallback policy |
| `SCENA_REQUIRE_HARDWARE_GPU` | Required native hardware-proof mode. Missing, CPU, lavapipe, llvmpipe, SwiftShader, and other software adapters fail instead of skipping. The manual hardware workflow sets it for PF01/PF02/FR06 evidence. | unset -> native GPU tests may use an available diagnostic adapter or skip when unavailable |
| `SCENA_REQUIRE_GPU_PARITY` | Required physical CPU/GPU parity mode for transmission, clipping, dynamic transforms, PBR, and PF08 texture baking. Missing or software adapters fail and every passing result records assertions, adapter/backend, commit, and source checksums. | unset -> a configured software adapter may run diagnostic conformance, otherwise the test emits non-release skip evidence |
| `SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS` | Diagnostic-only PF01 switch permitting one requested browser backend to produce a partial artifact. Partial artifacts always record `complete_backend_set=false` and `release_evidence=false`; required workflows must not set this flag. | unset -> PF01 requires both WebGPU and WebGL2 |
| `SCENA_HARDWARE_PROOF_COMMAND` | Records the literal native hardware-proof command in PF01/PF02 artifacts. Required hardware workflows set it to the command they execute. | canonical local command label |
| `SCENA_HARDWARE_PROOF_ROOT` | Overrides the workspace root beneath which standalone native PF01/PF02/FR06 executables write `target/gate-artifacts`. The one-shot Windows hardware-proof runner sets it to its verified proof workspace. | current process directory |
| `SCENA_BROWSER_COMPRESSED_ASSETS` | Enables the optional compressed-asset browser proof when set to `1`. | disabled |
| `SCENA_BROWSER_OVERSIZED_TEXTURE` | Enables the optional oversized-texture browser proof when set to `1`. | disabled |
| `SCENA_BROWSER_FORCE_REBUILD` | Forces the M6 browser probe to rebuild its task-local WASM package before launch, preventing a focused run from reusing stale generated bytes. | unset -> reuse an existing package when permitted by the selected workflow |
| `SCENA_BROWSER_VIEWER_ELEMENT_ONLY` | Runs only the viewer-element branch of the M6 probe when set to `1`. | full M6 probe |
| `SCENA_BROWSER_REQUIRE_V3D` | Requires the scene-host proof to use V3D hardware when set to `1`. | no V3D-specific requirement |
| `RUST_TOOLCHAIN` | Rust toolchain used by the scene-host browser proof's WASM build. | `1.93.1` |
| `SCENA_SKIP_WASM_BUILD` | Reuses a prebuilt scene-host WASM package when set to `1`; intended for scoped diagnostics with separately verified build provenance. | build WASM before proof |

To exercise the headline WaterBottle GPU render on an approved proof host:

```
SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1 \
  cargo test --test m8_real_asset_proof m8_real_asset_waterbottle_gpu_headline -- --exact
```

The GPU headline test fails when a working approved adapter is unavailable; it
does not fall back to the CPU renderer. The CPU proof is a separately named
test and artifact.

The required macOS Metal lane currently proves a live GPU render with more than
5,000 nonblack pixels, material color-family histograms, and fixed cap/body/
label/background region samples. Region checks normally use RGB Chebyshev
tolerance 25; the measured Apple Paravirtual Metal body sample permits up to 35.
It does **not** run the opt-in GPU reference comparison above.

The default lane independently runs
`q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders`.
It renders the real asset at 256x256 through the deterministic CPU renderer,
compares the live top-to-bottom opaque RGBA8 sRGB output with
`reference_cpu_256.png`, and proves the same oracle rejects flattened chrome,
a wrong material, and a wrong camera.

## Gate artifact locations

Render and capability artifacts land under `target/gate-artifacts/`. Tests
that emit artifacts always print the path to stderr so the human can open
them.

Notable locations:

- `target/gate-artifacts/m8-real-asset/waterbottle_gpu.png` and
  `waterbottle_gpu_result.json` — strict GPU headline output and typed result.
- `target/gate-artifacts/m8-real-asset/waterbottle_cpu.png` — separately gated
  CPU release-quality output.
- `target/gate-artifacts/q01-waterbottle-cpu/` — always-on 256x256 CPU live
  frame, three rejected mutation frames, and the provenance-bearing result.
- `target/gate-artifacts/m8-real-asset/waterbottle_gpu_renderer.toml` and
  `waterbottle_cpu_renderer.toml` — renderer-specific companion metadata;
  `waterbottle_renderer.toml` is only a latest-run compatibility pointer.

## Doctor

`cargo run -p xtask -- doctor --full` is the source of truth for "is the
codebase in a shippable shape". Failing doctor blocks release-readiness.

Doctor's truth substrings pin contract text in specific files. When the
underlying file is rewritten (Stage C2 moved glTF parsing from a hand-rolled
walker to the `gltf` crate's typed accessors), doctor's pinned strings need
updating in lockstep.
