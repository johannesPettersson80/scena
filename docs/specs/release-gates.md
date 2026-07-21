# Release gates

Status: active release contract

This document names the durable release evidence and commands that the release
workflow and `xtask` validate. A local green command is evidence only for the
exact source commit recorded by its artifact; it is not publication proof.

Static doctor guards enforce ownership and wiring; runtime correctness remains
owned by executed focused tests and rendered evidence. A source pin cannot
replace the focused behavior, mutation, browser, or physical-hardware proof
named by the corresponding release contract.

`Cargo.toml` is the canonical public version source. The D01 doctor sweep
compares `Cargo.lock`, generated package and WASM-size metadata when present,
tracked demo/proof titles and cache busters, current docs.rs links, public
dependency instructions, examples, and the documented bundle builder. Numeric
historical evidence is exempt only under the source-owned
`HISTORICAL_VERSION_PATH_PREFIXES` list (`CHANGELOG.md`, versioned release
notes, reviews, checklists, and decisions); current onboarding/API surfaces are
never exempt.

## Required command families

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo check --examples`
- `cargo run -p xtask -- doctor --full`
- `cargo publish --dry-run`

Browser, GPU, rendered-output, and performance lanes add their own typed proof
artifacts. Required lanes fail closed when their renderer or hardware work is
unavailable; optional diagnostic lanes must say that they are non-release
evidence.

## Required WebGPU hardware parity

`SCENA_REQUIRE_PARITY=1 SCENA_BROWSER_BACKENDS=webgpu npm run browser:q01-parity`
is the required physical-hardware image-parity producer. It compares the live
renderer-owned WebGPU readback with the CPU oracle for the exact
`m6-identical-unlit-triangle-v1` fixture under a declared top-left opaque-sRGB8
normalization, exact dimensions, and a declared CPU-reference-derived two-pixel
gradient-edge mask for both channel errors and foreground IoU. Candidate edges
never expand the exclusion mask.

The evaluator requires Chebyshev-4 coverage of at least 99.5%, RGB RMSE no
greater than 2, a 99.5th-percentile channel delta no greater than 4, and
foreground IoU of at least 0.995. It also requires rejection of wrong-color,
geometry-shift, missing-object, vertical-flip, linear-as-sRGB, and
stale-reference mutations. Draw calls, submissions, nonblack pixels, and
adapter identity remain necessary diagnostics, but none is an image-parity
criterion.

The dedicated `scena.q01.required_webgpu_pixel_parity.v1` artifact contains
the CPU/GPU/diff PNG hashes, metric summary, worst-region bounding box,
adapter, command, commit, and source checksums. The aggregate
`scena.m6.rust_wasm_renderer_probe.v1` artifact embeds the evaluated result.
The release consumer independently requires that pixel result and its six
mutation rejections; setting a top-level status string on a smoke-only artifact
does not pass. Software adapters remain `software-conformance` evidence and
cannot satisfy a `hardware-release` lane.

The final Windows physical proof is built from a clean exact commit with
`scripts/build_windows_complete_hardware_bundle.sh <output.zip>`. The bundle
contains pinned browser packages and cross-compiled proof executables plus a
SHA-256 manifest. Run its `run-proof.ps1` once with the same 40-hex
`-SourceCommit` and an operator-provided `-UploadUrl`. That one command covers
live Q01 WebGPU parity, combined WebGPU/WebGL2 output and semantic-AOV proofs,
attached-surface resize/loss/PresentOnly/MSAA, Q04 resource retirement, and the
controlled P01 shader-cache distribution. A dirty checkout or commit mismatch
is rejected before packaging, and every downloaded and installed file is
verified against the manifest.

## Required GPU resource lifecycle

The self-hosted Linux hardware workflow runs
`SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE=1 cargo test --test
c09_gpu_resource_lifecycle
required_hardware_gpu_resource_lifecycle_executes_complete_cycle -- --exact
--nocapture`. An unavailable or software adapter fails instead of passing by
returning before assertions.
The macOS Metal CI and release lanes run the same strict test on their physical
host before uploading the artifact set consumed by staging; this keeps the
release matrix satisfiable even when the optional self-hosted Linux runner is
offline.

The producer writes
`c09-gpu-resource-lifecycle/required-result.json` with schema
`scena.q04.required_gpu_resource_lifecycle.v1`. Release readiness independently
requires physical adapter provenance, at least ten executed lifecycle
assertions, a larger prepared resource set, return to the baseline retained
shape, `Confirmed` device polling, and exact destruction of every queued
resource with zero remaining pending work. The artifact is required for the
Linux native Vulkan release lane. Files written by the clearly named optional
developer smoke tests have `status:"skipped"` and
`proof_class:"optional-developer-smoke"`; they cannot satisfy this gate.
The checksum-verified Windows complete-hardware bundle runs the same strict
Q04 test executable; its independent validator rejects a missing adapter,
incomplete assertions, a retained-shape mismatch, or any pending destruction.

## Performance timing policy

M9 always requires a valid distribution with the configured minimum sample
count and always blocks deterministic allocation-count or allocation-byte
regressions. Wall-clock timing enforcement depends on the measurement host:

- `strict-controlled` is the default and enforces the stored p95 frame/prepare
  thresholds. Use it on the isolated Hetzner builder or another stable,
  controlled performance host.
- `report-only-hosted` is mandatory on shared GitHub-hosted runners. The
  artifact retains the observed pass/fail result and regression percentage,
  but variable wall-clock timing alone does not fail the lane.

The policy is selected by `SCENA_M9_TIMING_POLICY`, recorded in every benchmark
artifact and baseline-comparison row, and doctor-enforced in hosted workflows.
A hosted regression must be reproduced under `strict-controlled` before it is
called a product performance defect. Allocation regressions remain blocking in
both modes; the hosted policy is not permission to widen stored baselines.

## Required release artifacts

The release bundle includes provenance-bearing `m5-benchmarks.json` and
`m5-public-api-freeze.json` records. Their producer command, source commit,
toolchain/profile, timestamp, and content digest must be validated before the
artifacts can satisfy release readiness. Staging copies validated evidence; it
does not synthesize a passing result.

Their explicit production prerequisite is
`SCENA_RELEASE_COMMIT=$COMMIT SCENA_RELEASE_PROFILE=test-unoptimized cargo test --test m5_release`.
CI and the release workflow run that command after the broad workspace test so
the final uploaded M5 artifacts are not accidental leftovers from another test
invocation. Each artifact records `producing_command`, `toolchain`, `profile`,
`commit_sha`, `timestamp_unix_seconds`, non-empty `source_checksums`, and a
positive `sample_count`. Its `payload_sha256` is SHA-256 over the normalized,
compact JSON object after removing only `payload_sha256`; staging recomputes and
compares it.
The current `sample_count: 1` means one deterministic gate observation, not a
performance distribution or statistical benchmark claim.

`doctor --full` validates the durable source and documentation contract but
does not require or generate ignored `target/gate-artifacts` files. Artifact
production is the explicit prerequisite above, and release staging is the
separate fail-closed consumer.

## Readiness invocation and result

Readiness consumes one explicit staged bundle. Invoke it with either:

```bash
cargo run -p xtask -- release-readiness --artifact-root target/gate-artifacts
```

or a non-empty `SCENA_RELEASE_ARTIFACT_ROOT`. The CLI argument takes
precedence. A missing, empty, nonexistent, unreadable, or incomplete root is a
policy failure; discovery of zero files can never be reported as ready.

The command prints a `scena.release_readiness.v1` JSON result and exits nonzero
on failure. The result includes `artifact_root`, `artifact_root_source`,
`discovered_artifact_count`, `required_artifact_count`, and
`validated_artifact_count`. A passing result requires the validated count to be
positive and equal to the canonical required-artifact count, with every schema,
lane/backend, source-commit, timestamp, digest, and file-binding check green.
The required inventory and its specialized validators are owned together by
`crates/xtask/src/app/release/review_artifacts.rs`; in particular,
`m9-platform/linux-native-vulkan/rendered-output.json` and
`c09-gpu-resource-lifecycle/required-result.json` are required to exist.

`stage-release-artifacts <downloaded-root> <canonical-output-root>` already
names both its input and output explicitly. Staging success does not replace
the readiness command and does not make a partial bundle acceptable.

Human review remains normal repository governance but is not a machine release
artifact or publication prerequisite. Optional supplementary review evidence
is described in `docs/specs/release-reviews.md`. GitHub workflow status, a Git
tag, a GitHub release object, and registry publication are separate downstream
facts.

## Workflow dependency policy

Third-party GitHub Actions are pinned to lowercase 40-hex commit IDs. Each
`uses:` line retains the resolved release version in an adjacent comment so a
reviewer can understand the intended upgrade. `doctor --full` scans every YAML
file under `.github/workflows/` and rejects mutable references or immutable
references without a reviewable version comment. Local `./` actions and
`docker://` references are outside that third-party action rule.

`.github/dependabot.yml` opens reviewed weekly `github-actions` update pull
requests; updates must refresh both the commit and version comment. These pins
are preventive supply-chain hardening. They are not evidence or an allegation
that any previously referenced action tag was compromised.
