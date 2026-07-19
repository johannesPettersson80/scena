# Full-repo review v1.7.2 verification and remediation checklist

Created: 2026-07-16

Source review: `docs/reviews/full-repo-review-v1.7.2.md`

Checklist status: **complete; every required remediation is closed by implementation or an explicitly approved disposition**

Canonical charter: `docs/RFC-rust-3d-renderer.md`

This document is the source-truth audit and execution checklist for the review.
It separates verified defects from corrected claims, refuted claims, measured
performance work, and optional feature proposals. A checked box in the audit
sections means only that the claim was verified; it does **not** mean the
remediation is complete. Remediation boxes remain unchecked until their stated
proof and gates exist.

## 0. Audit boundary and non-negotiable corrections

### 0.1 Source identity and bootstrap proof

- [x] Canonical primary checkout identified as
  `/home/johannes/projects/scena`.
- [x] Audit destination identified as the same checkout,
  `/home/johannes/projects/scena`; canonical and destination `AGENTS.md` and
  `.codex/skills/**` therefore match by same-path identity.
- [x] Local branch recorded as `main`.
- [x] Local HEAD recorded as
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`.
- [x] Local relationship recorded as `main...origin/main [ahead 14]` during
  the audit.
- [x] User-owned worktree state preserved: the review file was untracked and
  no pre-existing file was modified by the audit.
- [x] The required skills were read in repository-mandated order: RFC
  governance, renderer architecture, glTF/assets, renderer quality, doctor,
  release hygiene, remote builder, and Git/GitHub.
- [x] The canonical RFC was read before classifying scope or feature work.
- [x] Heavy Rust validation stayed off the local machine.

### 0.2 Review provenance correction

The review's phrase "at commit `bea2a36` (v1.7.2)" is not precise enough.
At audit time, `git describe --tags` identified the source as
`v1.7.2-14-gbea2a36`: the package metadata still says 1.7.2, but the reviewed
source is fourteen commits after tag `v1.7.2`.

- [x] Amend the review header to say: **source snapshot `main@bea2a36`, Cargo
  package version 1.7.2, 14 commits after tag `v1.7.2`**.
- [x] Do not call the review a review of the tagged `v1.7.2` release unless it
  is rerun against the tag itself.
- [x] Attach reproducible evidence for process claims such as "eight parallel
  passes," independent spot-checking, and "verified live": prompt/result
  artifacts, exact command, fixture hash, backend, stdout artifact, exit code,
  and commit.
- [x] Replace universal competitive claims such as "no competitor has it,"
  "nothing like it exists anywhere," "single most common blocker," and "every
  serious CAD viewer" with either:
  - a dated feature matrix sourced from official product documentation; or
  - a qualified statement limited to the exact surfaces reviewed.
- [x] Reconcile the contract-count wording. `schema_entry_rows()` exposes 45
  catalog entries, but source contains additional versioned schema literals;
  at minimum the public v1 contracts emitted by the CLI help and version
  surfaces are missing from the catalog.

Review-provenance correction ledger (2026-07-17):

- `source identity`: the title and header now identify
  `main@bea2a36`, Cargo package 1.7.2, and
  `v1.7.2-14-gbea2a36`; the text explicitly says it is not a review of the
  tagged release.
- `process claims`: unsupported assertions about eight passes, independent
  spot-checking, and universal verification were withdrawn because the
  original review retained no prompt/result artifacts capable of proving
  them. Reproducible evidence that does exist is linked to this remediation
  checklist's per-item command/fixture/backend/provenance ledgers rather than
  being retroactively invented.
- `competitive claims`: universal comparisons, product-wide uniqueness,
  unmeasured Draco prevalence, and universal CAD-capping language were removed
  or narrowed to the exact scena surfaces inspected. No external feature
  matrix is claimed.
- `contract count`: the review now attributes 45 specifically to
  `schema_entry_rows()` and records that additional public versioned literals,
  including CLI help/version, remain outside that catalog pending FR01/FR04.

### 0.3 Verdict legend

- **Confirmed**: the central defect/gap exists, though wording, line numbers,
  severity, or proposed fix may still need correction.
- **Partial**: a narrower defect exists, but a material part of the claim is
  false, stale, already covered, or unsupported.
- **Refuted**: the implementation is correct for the stated contract/spec;
  applying the proposed review fix would be harmful.
- **Proposal**: a potentially valid roadmap item, not a current correctness
  failure. It must pass RFC/scope and demand review before implementation.

## 1. Complete claim-verification ledger

### 1.1 Bugs B1-B22

Audit result: **19 confirmed, 1 partial, 2 refuted**.

| ID | Verdict | Verified source truth | Required disposition |
|---|---|---|---|
| B1 | Confirmed | `Color::from_hex` and `from_hex_srgb` byte-slice a UTF-8 string after checking byte length only. The public recipe validator reaches the parser. Remote CLI proof panicked on `"€abc"`. | Fix under C01 with API and CLI no-unwind tests. Qualify the WASM-abort severity by panic/runtime configuration. |
| B2 | Confirmed | `replace_import` marks the old import stale, never removes its roots, and cannot subsequently remove it through the public stale-gated API. Failed replacement can also leave partial new nodes. | Replace with an atomic import ownership transaction under C02. |
| B3 | Confirmed | Embedded glTF images use global `memory:image-{index}` keys, so separate assets can collide. A cache hit can overwrite provenance bytes while retaining the prior decoded pixels. | Add stable per-source identity/digest and provenance invariants under C03. |
| B4 | Confirmed core | Valid non-normalized integer `POSITION` data allowed by `KHR_mesh_quantization` is rejected as if POSITION were absent. The NORMAL subclaim needs correction: unsupported invalid encodings should fail structurally, not default to +Z or be advertised as valid. | Implement the permitted POSITION cases and precise errors under C04. |
| B5 | Confirmed | CUBICSPLINE morph-weight output is chunked at three times the required width, so the sampler returns `None` and animation silently freezes. | Fix layout and validate imported clips under C04. |
| B6 | Confirmed | Multi-primitive glTF meshes bind weight animation to the Empty transform parent while renderable children own the weights. | Separate transform and morph target bindings under C04. |
| B7 | Confirmed core | Non-finite orbit, pan, touch, direct scene transform, and alignment paths can store NaN and corrupt camera/scene state. The claim that all other paths validate is false. | Enforce one finite-transform invariant at every public mutation boundary under C06. |
| B8 | Confirmed | Pan uses fixed world X/Y instead of view right/up after orbiting. | Add directional camera-space pan behavior and tests under C06. |
| B9 | Confirmed, broader | Missing `end_seconds`, end beyond clip duration, and start beyond duration lack a single defined clamp/validation policy and can generate repeated failures. | Define effective timeline bounds under C08. |
| B10 | Confirmed | Lazily created post, MSAA, depth-color, and pipeline resources are omitted from stats and destruction accounting. | Move creation to prepare and use exact owner accounting under C09/PF01. |
| B11 | **Refuted** | Anchor and connector locals intentionally remain in import units and are composed with unit-scaled node/world transforms. Converting them to meters would double-convert. | Do not implement the review fix. Add regression locks under C05; fix the different hierarchical scale bug N03. |
| B12 | Confirmed, broader | Numeric handles omit a kind tag. Node/import values collide immediately, and generation thresholds can eventually misclassify other recycled handles. | Add explicit namespace tags or a tagged registry under C07. |
| B13 | Confirmed | `filter_map` drops morph targets without POSITION, shifting target indices; tangent deltas have no representation. | Preserve target cardinality and semantics under C04. |
| B14 | Confirmed | Batched transforms mutate plain nodes before resolving stale instance roots; the operation is not atomic. | Resolve/validate all operations before any transition cancellation or mutation under C06. |
| B15 | **Refuted** | The KTX2 DFD/slot color-space mismatch error follows `KHR_texture_basisu`'s additional requirements. | Do not weaken it. Improve diagnostics and add compliant positive fixtures under C03. |
| B16 | Partial | Direct `GeometryDesc::polyline` panics on fewer than two points, but recipe validation already rejects that input before construction. | Add a fallible direct constructor and keep recipe defense-in-depth under C01. |
| B17 | Confirmed, understated | Direct deletion of either measurement/callout-generated child can orphan its sibling; callout deletion can also leave annotation state. | Enforce ownership closure or reject child deletion under C10. |
| B18 | Confirmed, broader | Hand-written JSON misses all U+0000-U+001F escaping. Raw `println!` can panic on EPIPE in both `scena` and `scena-convert`, not only the cited line. | Centralize serde JSON output and BrokenPipe handling under C01. |
| B19 | Confirmed | The wasm branch in `render/gpu/lifecycle.rs`, not `gpu/readback.rs`, reports polling/destruction completion without polling and clears pending work. Browser lifecycle proof trusts it. | Model automatic/unsupported/confirmed completion honestly under C09. |
| B20 | Confirmed; impact unmeasured | Imported clips bypass validation, and integer skin weights are dequantized but not vector-renormalized. Valid integer glTF constrains raw sums, so the remaining error may be small, but the specification still calls for normalization after quantization. | Define imported static-clip policy, validate all channel shapes/times, and normalize valid nonzero weights under C04. |
| B21 | Confirmed, broader | The GLB snippet does not compile; the first scene positions the camera inside the cube; docs and README pin stale 1.5 dependencies; snippets are not compile-gated. | Repair and compile/render-gate onboarding under C11. |
| B22 | Confirmed | Two checklist regions retain `[deferred]`/`[reopened]` status for shipped LTC area lights, tiled culling, and SSR. Doctor does not catch reverse status drift in this file. | Reconcile all duplicate statuses and add reverse-drift enforcement under C11/D02. |

Normative sources for the two spec-sensitive corrections:

- [`KHR_mesh_quantization`](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_mesh_quantization/README.md)
  permits the quantized POSITION encodings the loader currently misses.
- The [`KHR_texture_basisu` additional requirements](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_texture_basisu/README.md#additional-requirements)
  require KTX2 DFD color-space information to match color versus non-color
  texture usage; B15's proposed relaxation is prohibited.
- The [glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
  is the authority for morph-target cardinality, animation output layout, and
  quantized skin-weight constraints.

### 1.2 Proof-integrity claims S1-S10

Audit result: **3 confirmed, 7 partial**. Every partial item still contains
actionable proof debt; the correction prevents deleting proof that already
works or overstating the affected CI surface.

| ID | Verdict | Verified source truth | Required disposition |
|---|---|---|---|
| S1 | Partial | Default `cargo test` skips live WaterBottle renders and the Blender agreement test compares committed images. CI does run a live macOS Metal WaterBottle render with nonblack/color-family and region checks, but no workflow enables the live golden-diff mode. The review's uniform ±35 and “CI renders nothing” language is false. | Add a default live CPU golden; accurately classify the GPU oracle under Q01. |
| S2 | Partial | The cited Rust tests evaluate committed PNGs, but WebGL2 CI already crops the live proof canvas and applies DeltaE/material-specific gates. The missing surfaces are live CPU and required WebGPU/mobile metrics. | Preserve WebGL2 and extend/reuse its evaluator under Q02. |
| S3 | Partial | m3a/m3b and measurement have weak nonblack-style oracles. m7 also asserts before/after difference, m3a asserts a logical pick, and measurement uses `>40`, so the literal grouping is overstated. | Add feature-specific region/differential mutation tests under Q03. |
| S4 | Confirmed | m1/m3a/m3b browser tests prove Canvas2D roundtrips of CPU frames; m6 renders WebGL2 but never compares a CPU frame. The former targets are not explicitly run by workflows. | Build real CPU-WebGL2 parity and activate/consolidate dormant targets under Q04. |
| S5 | Partial | The reference test samples three locations and parses an unused hash, but other m2 effect tests contain additional local or full-region checks. Shadow/IBL and broad spatial behavior remain weak. | Add per-effect footprint and known-bad tests under Q05. |
| S6 | Confirmed, broader | Directory-prefix “retired” exemptions swallow missing active docs; even the canonical RFC is treated as retired. Other helpers fail open on unreadable targets or use an overbroad WGSL sibling fallback. | Make document/source contracts fail closed under D02. |
| S7 | Partial | The required WebGPU job can be green with `ALLOW_UNAVAILABLE`, and doctor positively pins that flag. The downstream premerge staging job should still fail the complete workflow because it requires a passed nonblack WebGPU result. | Make the required lane itself strict; do not rely on downstream reinterpretation. See Q06. |
| S8 | Partial | The named Linux m9 lane can silently downgrade to CPU and pass. Other lavapipe parity and label tests are strict, so a general Linux GPU-init regression is not “zero red anywhere.” | Make the Linux native release lane strict and keep fallback only in a named diagnostic lane under Q06. |
| S9 | Partial, worse in one area | Live CLI SSIM has no positive live-render acceptance, though synthetic positive SSIM tests exist. The advertised `icc` feature appears to be dependency-only with no implementation. Env/ignore scans are non-recursive and miss the ignored external Cardine fixture. | Add live SSIM proof; implement or remove ICC; recursively audit flags/ignores under Q07/D02. |
| S10 | Confirmed at audited HEAD | The three production files and one cfg-test file exceed the 500-line rule. A truly fresh isolated snapshot also lacks two ignored gate artifacts, so `doctor --full` reports eight findings, not only four. The cfg-test file exposes inconsistent scanner semantics. | Fix scanner semantics first, split three production modules, and remove ignored-artifact dependence under D03. |

The review's doctor structural note is **partial**: doctor uses many substring
pins, but it also parses links/JSON, validates schemas and hashes, scans source
structure, and computes some artifact metrics. More importantly, its roughly
249 xtask tests are not run by current CI commands; workflows call xtask
commands but do not run `cargo test -p xtask` or `cargo test --workspace`.

### 1.3 Performance claims P1-P12

Audit result: the structural hot paths are largely real. Most byte counts,
speedups, "worst" rankings, and elapsed-time claims are estimates and must not
be presented as measurements until PF00 exists.

| ID | Verdict | Verified source truth | Required disposition |
|---|---|---|---|
| P1 | Confirmed algorithm; impact unmeasured | Shadow rays linearly scan all occluders; area lights add 16 samples. Expensive scans still depend on relevant shadow/light state. The `~1e9` and `~6x` figures are estimates. | Instrument first, then deterministic BVH/TLAS work under PF06. |
| P2 | Confirmed; multipliers unmeasured | Full prepare creates multiple primitive lists and CPU draw clones primitives/strokes/labels. Exact `380 B`, `150-200 MB`, and `40 MB` claims were not measured. | Remove frame clone first, then redesign shared prepared data under PF03. |
| P3 | Confirmed with count corrections | Asset getters deep-clone descriptors and CPU texture sampling locks storage per sample. Some stated clone/sample multipliers are conditional. Changing the existing getter return type is semver-sensitive. | Add compatible immutable snapshot/Arc accessors and resolve textures outside loops under PF04. |
| P4 | Confirmed | World transforms allocate/walk ancestors repeatedly and visibility repeats the walk. Cache invalidation also needs visibility revision, omitted by the review. | One top-down transform/visibility cache under PF05. |
| P5 | Confirmed recurrence; proposed fix incomplete | Tangents are regenerated after deformation/world transform. A once-per-geometry cache is unsafe for nonuniform scale, morphs, skinning, and mirrored transforms. | Separate static and deformed tangent policies under PF07. |
| P6 | Confirmed plus correctness defect | Picking is brute force despite its BVH module claim and clones base geometry per event. It also ignores current morph/skin deformation. | Fix pose correctness first, then local-ray/AABB/BVH acceleration under C12/PF06. |
| P7 | Confirmed core | Qualifying CPU/fallback textured paths subdivide each triangle 48x48; “any textured material” is false. Several invariants and transmission work sit inside hot loops. | Hoist/gate first, then bounded adaptive subdivision under PF08. |
| P8 | Confirmed serial work; elapsed cost unmeasured | Environment baking and mesh preparation contain serial parallelizable loops. Sidecar hits avoid some work; native rayon needs deterministic and WASM policies. | Parallelize only after PF04 removes inner-loop locking under PF09. |
| P9 | Confirmed, urgent | Native surface rendering shares a synchronous readback path and blocking polls. Current synchronous getter cannot decide retrospectively whether a frame was consumed. | Split present, sync capture, and async readback APIs under PF02. |
| P10 | Confirmed, RFC violation | Post/MSAA resources and pipelines are created during `render()`, violating the explicit prepare/render charter. | Move creation to prepare/output-prepare and instrument zero render-time creation under PF01. |
| P11 | Mostly confirmed/partial details | Linear animation scans, redundant inversions, instance dedup, import rebind, data-URI keys, and absent-attribute vectors are real. Only two of four inversions are clearly redundant; typed WASM transform batching already exists; `Vec::new()` alone does not allocate. A separate draw-uniform scan is potentially O(T²). | Address measured subitems under PF10; retain compatibility APIs. |
| P12 | Partial/materially overstated | The repo has M9 distributions/allocation counts, label timing, workflow timing, high-instance rows, and 4K lanes. Missing are representative P1-P11 workloads, prepare gates, bytes copied, honest thresholds, and dynamic-update rows. M5 hardcodes allocation bytes to zero and M9 baselines permit 100% while artifacts advertise 5%. | Repair benchmark truth before optimization under PF00. |

### 1.4 Feature proposals F1-F13

These are roadmap candidates, not release-blocking bugs. Every public API,
scope, or milestone decision begins with `scena-rfc-governance`; renderer
implementation then uses the architecture/area/quality/doctor/release skills.

| ID | Verdict | Verified source truth | Governance disposition |
|---|---|---|---|
| F1 | Confirmed gap | `schema get` lacks field-level constraints and no `vocab` command exists. | Agent-surface candidate FR01. Generate from one source of truth. |
| F2 | Confirmed gap | Build manifest exists internally, but only `recipe render` exposes it and requires output PNG. | Candidate FR02. Prefer a clear `recipe build` verb over an ambiguous dry-run. |
| F3 | Confirmed gap with contract correction | `place` emits a transform preview only. `visual_patch.v1` carries ephemeral host handles and is unsafe as the default persistent artifact. | Candidate FR03. Emit updated recipe or a recipe-ID patch. |
| F4 | Confirmed gap | Help lacks per-command output schemas; load failures are polymorphic; policy roots are not queryable; onboarding is broken. | Candidate FR04, with B21 correctness work first. |
| F5 | Partial/stale | Generic views/turntable/clip capture is absent, but `recipe inspect-cad` already renders three views and a contact sheet. | Extract/reuse existing capture under FR05. |
| F6 | Confirmed gap; effort understated | No semantic ID/depth/normal AOV contract exists. CPU-first may be moderate; cross-backend semantics, transparency, labels, instances, and MSAA are not cheap. | Medium/large candidate FR06 with explicit semantics. |
| F7 | Confirmed gap; uniqueness refuted | Aggregate capture diff exists, but no structural or node-attributed CLI diff. Structural diff does not depend on F6; attribution does. | Split FR07 into independent structural and F6-dependent attributed slices. |
| F8 | Confirmed reserved surface; effort understated | Recipe validator fails anchors/connectors/bounds/named states closed, while runtime owners exist. Identity, validation, manifest mapping, and state snapshot semantics still need design. | Candidate FR08 after contract design. |
| F9 | Confirmed Draco/mode gaps; ranking unverified | Draco is deferred/external; non-triangle primitive modes hard-error. “Most common” and decoder availability are not proven. Native WebP must be fixed before extension rebinding. | Demand- and dependency-gated FR09. |
| F10 | Confirmed gap; readiness overstated | Capability/diagnostic rows exist, but punctual light authoring/defaults for shadow maps do not. Spot and point shadows are distinct projects. | Split FR10 into spot then cubemap point-shadow scopes. |
| F11 | Confirmed gap | Weighted-blended OIT is CPU-only and GPU capability remains disabled. | Large visual/backend project FR11. |
| F12 | Confirmed absence/deferred; scope unresolved | No exporter exists and the checklist defers it. Export is not explicitly in the canonical RFC's in-scope list. | RFC ratification first; evaluate companion crate/tool in FR12. |
| F13 | Mixed/partial | Section capping, KTX2 cubemap environments, higher-precision capture, watch, and international text are gaps. The claimed public `LabelDesc::sdf()/msdf()` APIs do not exist. “Complex script” understates font coverage/fallback/bidi/line-breaking needs. | Keep separate FR13a-f items; watch depends on C02. |

## 2. Additional findings missed by the review

These findings are first-class remediation work, not footnotes to the original
priority order.

| New ID | Severity | Finding | Primary owner / dependency |
|---|---|---|---|
| N01 | Critical/High | Production glTF loading uses `Gltf::from_slice_without_validation`; malformed child/mesh/skin/accessor/sampler references can reach upstream unwraps and panic on untrusted files. Cycles/DAG violations can also create incorrect imports. | `assets`; C01 before other importer work. |
| N02 | High | Import instantiation mutates the scene incrementally. Late anchor/connector/child/skin errors leave partial nodes; replacement stales the visible old import before the fallible operation. | `scene/import`; C02. |
| N03 | High | Unit conversion multiplies dimensionless local `scale` on every descendant and scale-animation sample, compounding unit factors through non-meter hierarchies. Existing tests check local values, not world truth. | `scene/import/options`; C05. |
| N04 | High | Native WebP is accepted and advertised but `decode_texture_pixels` returns `Ok(None)`; native rendering silently lacks decoded pixels while browser/WASM has a separate path. | `assets/texture`; C03. |
| N05 | High | Picking intersects base geometry and ignores current morph and skin deformation, so selection can disagree with the rendered pose. | `picking` + animation/skin; C12. |
| N06 | High/Potential O(T²) | `draw_uniform_index_for` linearly scans prior uniforms for every primitive; many unique transforms can make GPU preparation quadratic. | `render/gpu/vertices`; PF10 after PF00 measurement. |
| N07 | Critical proof integrity | Release staging accepts absent/`local-checkout` provenance, then inserts the current commit and a fresh timestamp. Stale or unattributed evidence can emerge labeled current. | `xtask release`; D01. |
| N08 | Critical proof integrity | WaterBottle staging accepts PNG magic plus >1 KiB, hardcodes `rust_test_output_observed=true`, and has a positive test using an invalid fake PNG. | `xtask release`; D01/Q01. |
| N09 | High proof integrity | Roughly 249 xtask/doctor mutation tests are not run by CI/release; root-package `cargo test` does not cover workspace member `xtask`. | workflows; D01. |
| N10 | High proof integrity | Staged visual-proof validation does not strongly bind schema, lane, proof class, command, release-evidence status, source hashes, or renderer-owned output. | `xtask release`; D01. |
| N11 | High proof integrity | Browser release acceptance can be satisfied by generic matching-backend, passed, nonblack JSON and may fall back to canvas evidence. | browser probe + xtask; D01/Q04. |
| N12 | High governance | Doctor explicitly treats the canonical RFC as retired and does not require it, so deleting the architecture authority can pass docs checks. | `xtask doctor`; D02. |
| N13 | High contract truth | Cargo advertises `icc`, docs claim ICC behavior, but no implementation or tests were found. | render quality/assets + release metadata; Q07. |
| N14 | Medium/High API truth | `SceneHostCore::headless_gpu_with_fetcher` silently falls back to CPU despite its strict-sounding name. Backend is inspectable later, but constructor success can hide missing GPU proof. | `scene_host`; C13/Q06. |
| N15 | Documentation | `README.md` also pins `scena = "1.5"`, beyond the stale getting-started pin named in B21. | docs; C11. |
| N16 | Release hardening | GitHub Actions use mutable major-version tags rather than immutable commit SHAs. | workflows; D04. |
| N17 | Contract discovery | Public CLI help/version v1 contract literals are omitted from the claimed public schema catalog. | schema catalog/doctor; FR01/FR04. |
| N18 | Operational | The remote-builder path mandated by repository instructions, `$HOME/projects/scena`, did not exist during the audit. An isolated snapshot worked, but the documented shared path and real builder state are out of sync. | repo instructions/builder provisioning; O01. |
| N19 | Documentation/proof operations | `CLAUDE.md` describes an obsolete WaterBottle GPU switch/fallback, stale artifact filenames, omits browser JavaScript flags, and calls an RGB Chebyshev comparison DeltaE. | contributor docs; C11/Q01/D02. |
| N20 | High proof integrity | `doctor --full` depends on ignored `target/gate-artifacts` files with no durable provenance; a fresh checkout reports four extra missing/read failures. `doctor --architecture` also pulls in those unrelated release-artifact requirements. | doctor/release artifacts; D03/PF00. |
| N21 | Critical proof integrity | Release staging authors every required role-review report with `blocker_status: clear`, writes an empty findings register, and generates maintainer sign-off plus `decision = "approve"` from `GITHUB_ACTOR`/automation without consuming reviewer evidence. | `xtask release`; D01 before trusting release readiness. |

## 3. Execution rules for every remediation item

These rules apply to every unchecked item below.

- [x] Start from the canonical RFC and confirm owner-module/scope before adding
  public API or changing milestones.
- [x] Re-run branch/worktree bootstrap after every branch switch or new
  checkout. Manually copy root `AGENTS.md` and complete `.codex/skills/**`,
  hash-verify them, and report canonical path, destination path, branch, HEAD,
  and result before edits or gates.
- [x] Write the narrowest deterministic test first. Run it on the remote
  builder and confirm it fails for the expected old behavior before production
  changes.
- [x] Make the smallest owner-module change that passes the focused test.
- [x] Add a structured error instead of panic, silent fallback, fabricated
  success, or stringly namespace behavior.
- [x] For every repeatable/source-detectable failure family, add or strengthen
  `xtask doctor` coverage and a known-bad mutation fixture.
- [x] Use rendered-output proof for visual behavior; unit tests alone cannot
  close camera, material, deformation, shadow, post, parity, label, or overlay
  items.
- [x] Use a real GPU lane for hardware/backend claims. The CPU builder can
  compile and run deterministic CPU/lavapipe proofs but cannot establish
  hardware-specific visual correctness.
- [x] Record the validation ledger per logical unit:
  `focused`, `scoped`, `full`, and `skipped` with exact commands/results.
- [x] Do not run the full release chain after every small patch. Run it once at
  a natural cross-backend/public-API/release checkpoint, and reuse unchanged
  evidence honestly.
- [x] Do not commit, push, tag, merge, close issues, or publish unless the user
  explicitly authorizes that separate Git/GitHub action.

## 4. Gate and proof-integrity remediation

Proof infrastructure must become trustworthy before it is used to close the
large correctness/performance batches.

### D01 — Make release staging preserve and verify source evidence

Scope: N07-N11, N21, and parts of S1/S4/S7/S8.

- [x] Add `cargo test -p xtask` to CI before `doctor --full` and to the release
  workflow; do not rely on root-package `cargo test`/`--all-targets`.
- [x] Add one known-bad doctor/release fixture to that lane so a zero-test or
  wrong-package command cannot appear green.
- [x] Make every required source JSON carry its own exact, non-local commit,
  generation timestamp, schema, producing command/test, and source checksums.
- [x] Reject missing, blank, `local-checkout`, foreign, or stale provenance in
  release staging.
- [x] Never rewrite source `commit_sha` or source generation time. Record
  `staged_at`, staging checkout, and staging tool version in separate fields.
- [x] Decode WaterBottle PNGs with a real decoder; require expected dimensions,
  color type, nontrivial pixel distribution, and the approved comparison
  metrics—not only magic bytes and size.
- [x] Require the companion WaterBottle result/metadata artifact, exact test
  identity, approved backend/adapter policy, source PNG hash, command-record
  checksum, `release_evidence=true`, and no skip/fallback marker.
- [x] Turn the current fake-PNG positive fixture into a rejection fixture.
- [x] Define typed per-proof artifact validation: exact schema, lane, proof
  class, producer, source artifact hash, renderer-owned output, dimensions,
  checksum, commit, timestamp, and release status.
- [x] Delete staging behavior that authors role review reports, an empty
  findings register, maintainer sign-off, or an approval decision. Artifact
  staging may validate and copy review evidence; it must never perform the
  review or approve the release.
- [x] Require pre-existing reports for every required role, the complete
  findings register, and explicit maintainer sign-off, all bound to the exact
  commit and carrying independently verifiable reviewer identity/provenance.
- [x] Validate that required roles are distinct as policy requires, every
  finding is represented with status/history, no blocker remains open, and the
  sign-off references hashes of the exact reviewed reports/register.
- [x] For browser GPU claims, require
  `renderer_readback.source == "renderer-owned-gpu-copy"`; reject generic
  Canvas2D or arbitrary nonblack JSON as renderer proof.
- [x] Add negative fixtures for wrong backend, wrong lane, absent result,
  canvas-only readback, zero pixels, stale hash, substituted PNG, missing
  command, synthesized provenance, missing review role, open finding, review
  commit mismatch, synthetic automation reviewer, tampered report, and absent
  maintainer sign-off.
- [x] Update release-readiness reports to distinguish source generation from
  staging/aggregation and surface every rejected artifact with a stable code.

Acceptance:

- [x] Every negative fixture fails the focused xtask test for its intended
  reason.
- [x] A valid artifact passes without any provenance field being rewritten.
- [x] CI visibly runs xtask tests before consuming doctor/release output.
- [x] Staging cannot turn arbitrary bytes or stale local output into a
  `passed`, current-commit visual-proof contract.
- [x] Staging cannot create, clear, close, sign, or approve a review; the staged
  report hashes match independently produced inputs byte-for-byte.
- [x] Missing reviewers, automation-authored approval, open blockers, and
  tampered review inputs make release readiness fail closed.

Validation ledger (D01, remote isolated builder copy
`$HOME/.cache/codex-worktrees/scena-d01-release-evidence`, target
`$HOME/.cache/codex-targets/scena-d01-release-evidence`):

- `focused`: provenance, typed visual-proof, browser headline, WaterBottle,
  review-integrity, workflow mutation, archive-installer, Python archive, and
  JavaScript provenance fixtures were observed red before their owner changes
  and green afterward. The integration regressions were reproduced exactly;
  `tests_env_flags_documented_passes_when_flag_in_claude_md`,
  `xtask_module_split_is_source_enforced`, and all five
  `tests_09::release_lane_artifact_*` tests now pass.
- `scoped`: `cargo test -p xtask -- --skip
  app::tests_08::release_readiness_has_no_open_release_deferrals` passed 256
  tests with only the final external-evidence readiness assertion filtered;
  `cargo test --test scena_cli_schema` passed 4/4; `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `doctor --docs`
  passed.
- `full`: full M5 (6/6) and M9 (17/17) targets passed for the changed artifact
  producers. `doctor --full` now reports only the four pre-existing D03 module
  size findings and no D01 release/provenance finding.
- `skipped`: the final release-readiness success assertion remains intentionally
  unrun until independent reviewer evidence and final hardware/browser lane
  artifacts exist; no commit, push, tag, merge, release, or publish action was
  performed.

### D02 — Make doctor fail closed and syntax/ownership aware

Scope: S6, N12, N19, B22, recursive env/ignore weaknesses, missing
`docs/specs/release-gates.md`.

- [x] Replace directory-prefix retired-doc exemptions with an exact,
  reviewed allowlist containing an owner and removal rationale per file.
- [x] Remove `docs/RFC-rust-3d-renderer.md` from retirement logic and add it to
  required canonical documents.
- [x] Enumerate every active path passed to a doctor pin and require that it
  exists and is readable; do not silently accept missing specs, checklists,
  ADRs, API docs, or benchmarks.
- [x] Restore or deliberately replace `docs/specs/release-gates.md`, which is
  referenced by both release-hygiene instructions and doctor contracts.
- [x] Make `forbid_contains_path` fail on missing/unreadable input unless the
  exact target is allowlisted as retired.
- [x] Restrict Rust-to-WGSL sibling fallback to explicit shader marker
  contracts; never let arbitrary missing Rust text be satisfied by a shader.
- [x] Use syntax/module-aware checks for named test functions, cfg ownership,
  public items, and attributes. Reserve raw substring pins for prose/schema
  contracts where text is the contract.
- [x] Recursively scan Rust and JavaScript/TypeScript for environment flags,
  ignored tests, early returns/skips, and unsafe allow-unavailable modes.
- [x] Generate contributor env-flag documentation from the same registry or
  fail when docs/workflows/read sites diverge.
- [x] Add reverse status-drift detection: shipped source + accepted proof must
  not coexist with active `[deferred]` or `[reopened]` claims in any current
  checklist.
- [x] Add public-contract discovery that compares schema literals/typed
  contracts against the catalog and catches N17.

Mutation acceptance:

- [x] Deleting the RFC produces a canonical-doc finding.
- [x] Deleting the m2 checklist or any active pinned document produces a
  missing-file finding.
- [x] Typoing a pinned checklist path produces a finding.
- [x] Deleting a forbid-scan target produces a finding.
- [x] Deleting one exact retired allowlisted file is permitted and reported as
  intentionally retired.
- [x] A comment with a pinned test name does not satisfy a real-test contract.
- [x] A JavaScript `process.env` flag missing from docs is detected.
- [x] A shipped feature marked deferred in a duplicate checklist is detected.

D02 validation ledger (isolated Hetzner CPU builder):

- `focused`: `cargo test -p xtask app::tests_18:: -- --nocapture` passed all
  18 fail-closed/syntax/status/schema mutations, including the intentional
  retired-document report.
- `scoped`: `cargo test -p xtask -- --skip
  app::tests_08::release_readiness_has_no_open_release_deferrals` passed
  271/271; `cargo test --test scena_cli_schema` passed 4/4; `cargo fmt --all
  --check` and `cargo clippy -p xtask --all-targets -- -D warnings` passed;
  `doctor --docs` passed.
- `full`: not run for D02 because its only remaining architecture/full-doctor
  findings are the four source-size findings assigned to D03. A direct
  `doctor --architecture` run confirmed exactly those four findings and no D02
  finding.
- `skipped`: browser/GPU rendering was not needed for doctor/docs/schema
  enforcement. Required WebGPU workflow policy is source-checked fail closed;
  live lane execution remains Q06 evidence.

### D03 — Restore hermetic doctor architecture gates

Scope: S10, N20, and ignored gate-artifact dependency.

- [x] Add equivalent inline and external `#[cfg(test)]` module fixtures; prove
  doctor treats them consistently.
- [x] Decide and document whether the 500-significant-line production rule
  applies to test-only files. Prefer excluding module-graph-proven cfg-test
  files from the production architecture finding while retaining a separate
  test-maintainability rule if desired.
- [x] Split `src/bin/scena/args.rs` by command/argument ownership.
- [x] Split `src/scene/recipe/validation/expectations.rs` by expectation
  family/target owner.
- [x] Split `src/scene/recipe/validation/imports.rs` by import validation
  responsibility.
- [x] Re-evaluate `src/render/quality/tests.rs` only after scanner semantics are
  fixed; do not reorganize test code merely to appease an inconsistent rule.
- [x] Remove `doctor --full`'s dependence on ignored, locally pre-existing
  `target/gate-artifacts/m5-benchmarks.json` and
  `m5-public-api-freeze.json`.
- [x] Either generate those artifacts deterministically in the gate or make
  their provenance-bearing production command an explicit prerequisite.
- [x] Reject artifacts without commit, toolchain/profile, command, sample
  count, content hash, and generation timestamp.
- [x] Make submodes compositional: `doctor --architecture` should not fail on
  unrelated missing release artifacts unless its documented contract says it
  runs full release checks.

Acceptance:

- [x] A freshly rsynced checkout with empty `target/` has deterministic doctor
  behavior.
- [x] `doctor --architecture`, `doctor --docs`, and `doctor --full` report only
  their documented scopes.
- [x] `doctor --full` passes at the remediation checkpoint without relying on
  ignored workstation residue.

D03 validation ledger (2026-07-16):

- `focused`: an external `#[cfg(test)] mod tests;` fixture first failed with a
  501-significant-line production-size finding; after module-graph-aware
  exclusion, the equivalent external cfg-test and non-test controls passed
  2/2. The M5 staging mutation proof first accepted missing `toolchain`, then
  rejected missing `toolchain`, `profile`, `producing_command`, `sample_count`,
  `payload_sha256`, and post-hash content mutation after the validator landed.
  A full M5 run exposed and then closed an in-memory-versus-round-trip numeric
  JSON digest instability.
- `scoped`: remote `scene_recipe_contracts` passed 22/22, `scena_cli_schema`
  passed 4/4, `cargo check --bin scena --all-features` passed, full M5 passed
  6/6 with exact commit/profile provenance, `cargo fmt --all --check` passed,
  and `cargo clippy --workspace --all-targets -- -D warnings` passed. The
  module split guard, env registry guard, stage mutation test, and final
  no-open-deferrals assertion each passed independently.
- `full`: the manually bootstrapped isolated copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence` used the
  fresh task target `/home/johannes/.cache/codex-targets/scena-d03-fresh` after
  both its repo-local `target/` and task target were removed. Before any M5
  artifact existed, `doctor --architecture`, `doctor --docs`, and
  `doctor --full` all passed; the fresh-target xtask suite then passed 275/275.
  Canonical source remained `/home/johannes/projects/scena`, branch `main`, HEAD
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`; remote `AGENTS.md` and the full
  skill manifest matched the canonical checkout before proof.
- `skipped`: browser/GPU rendering was not relevant to scanner semantics,
  source-only module moves, or provenance validation. No commit, push, tag,
  merge, release, or publish action was taken.

### D04 — Harden workflow dependencies

- [x] Pin GitHub Actions to immutable commit SHAs while retaining version
  comments for readability.
- [x] Configure Dependabot/Renovate or an equivalent reviewed update path.
- [x] Add a workflow policy check rejecting new mutable action references.
- [x] Treat this as supply-chain hardening, not proof that any current action
  tag has been compromised.

D04 validation ledger (2026-07-16):

- `focused`: the workflow-policy mutation test first failed because
  `actions/checkout@v4` produced no immutable-pin finding. It now rejects a
  mutable tag, rejects a 40-hex pin without a release-version comment, and
  accepts `actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5
  # v4.3.1`.
- `scoped`: all workflow references were inventoried. Direct upstream Git refs
  resolved `actions/checkout` v4.3.1 to `34e114876b0b11c390a56381ad16ebd13914f8d5`,
  `actions/upload-artifact` v4.6.2 to
  `ea165f8d65b6e75b540449e92b4886f43607fa02`,
  `actions/download-artifact` v4.3.0 to
  `d3f86a106a0bac45b974a628896c90dbdf5c8093`, `actions/setup-node` v4.4.0 to
  `49933ea5288caeca8642d1e84afbd3f7d6820020`, and the
  `dtolnay/rust-toolchain` 1.93.1 branch to
  `841e5d09118f0311af26ce5ba303c1f15b358772`. CI and release workflows use
  only those immutable commits with adjacent version comments.
- `scoped`: `.github/dependabot.yml` now schedules reviewed weekly
  `github-actions` updates. `WORKFLOW-ACTION-PIN` scans every workflow YAML,
  requires lowercase 40-hex third-party revisions plus version comments, and
  pins the Dependabot contract. The M9 upload contract test and
  `cargo fmt --all --check` passed remotely.
- `full`: remote `doctor --full` passed and the complete xtask suite passed
  276/276 in the bootstrapped isolated validation copy using
  `/home/johannes/.cache/codex-targets/scena-d03-fresh`.
- `skipped`: GitHub workflow execution was not available without pushing the
  uncommitted tree; local syntax/source policy and remote Rust proof are
  recorded separately. No commit, push, tag, merge, release, or publish action
  was taken. Pinning is preventive hardening and is not a compromise claim.

## 5. Correctness and public-contract remediation

### C01 — Remove panic/invalid-output paths from untrusted input surfaces

Scope: B1, B16, B18, N01.

Test first:

- [x] Add Unicode/non-ASCII hex cases to both public color constructors and
  assert structured errors without unwind.
- [x] Add CLI fixtures for `validate-recipe`, recipe build/render, and WASM host
  validation using `"€abc"`; assert JSON diagnostics and stable nonzero exit,
  never panic text.
- [x] Add malformed glTF fixtures for child, mesh, skin, accessor, sampler,
  animation reference, cycles, and DAG/schema violations. Reproduce corrupt
  indices and cyclic graphs first through an isolated CLI/subprocess so a
  stack-overflow abort cannot kill the test runner; once validation rejects
  before traversal, add in-process no-unwind assertions requiring `AssetError`
  with stable paths/codes.
- [x] Add direct polyline 0/1-point tests and recipe validation/build
  defense-in-depth tests.
- [x] Add machine-output tests for newline, tab, every JSON control range,
  Unicode, closed-pipe/EPIPE, and non-BrokenPipe stdout errors in both binaries.

Implementation:

- [x] Parse hex through validated ASCII bytes/nibbles; perform no unchecked
  UTF-8 range slicing.
- [x] Replace production `from_slice_without_validation` with validated glTF
  parsing. If a compatibility pre-pass is indispensable, document the exact
  transformation and still run full validation before traversing accessors.
- [x] Ensure failed glTF parsing cannot mutate asset storage or scene state.
- [x] Add `GeometryDesc::try_polyline -> Result`; preserve/deprecate the
  panicking wrapper only as required by semver, and route recipe construction
  through the fallible form.
- [x] Serialize all machine JSON with serde.
- [x] Centralize buffered stdout writes; treat BrokenPipe as quiet success and
  other write failures as structured/nonzero failures.
- [x] Add doctor coverage forbidding unchecked production glTF parse,
  hand-written machine JSON escaping, and raw machine-output `println!`.

Done when:

- [x] Every malformed/untrusted fixture returns a stable error and leaves
  storage/scene revisions unchanged.
- [x] CLI stdout always parses as the declared schema when output is produced.
- [x] Neither binary panics when its consumer closes the pipe.

C01 validation ledger (2026-07-16):

- `focused`: the original Unicode CLI reproducer panicked while slicing
  `"€abc"`. Both public color constructors, recipe validation/build/render,
  native CLI validation, and the Node WASM runtime now return typed or declared
  JSON diagnostics without unwind or panic text. The Node `wasm-pack` lane
  passed 1/1.
- `focused`: malformed child, mesh, skin, accessor, sampler, animation, cycle,
  and multiple-parent DAG fixtures run first in isolated subprocesses and then
  in-process. They now fail before traversal with stable `AssetError` paths.
  The late-failure storage test first observed `materials_evicted = 1`; parsing
  now occurs against a cloned asset-storage transaction and commits only on
  success, and the regression passes with unchanged storage counters.
- `focused`: direct zero/one-point polyline construction and recipe
  validation/build reject input without unwind. `GeometryDesc::try_polyline`
  owns the fallible contract; the legacy wrapper remains only for semver, and
  recipe construction uses the fallible path.
- `focused`: both binaries round-trip newline, tab, U+0000 through U+001F, and
  Unicode through serde JSON. Closed Unix-stream peers produce quiet success;
  `/dev/full` produces exit 74 and a parseable `scena.cli_io_error.v1` stderr
  report. The scene-host CLI contract suite passed 4/4 and both binary unit
  suites passed 3/3 and 4/4.
- `scoped`: the glTF parser performs a documented JSON/GLB compatibility
  normalization only for empty animation arrays, empty material-variant
  declarations, and Scena-owned required-extension declarations that upstream
  gltf 1.4.1 cannot validate. Original extension declarations are preserved
  for Scena policy, full validation runs before traversal, and unsupported
  required extensions still fail typed. Fixture POSITION accessors exposed by
  strict validation were corrected with their real bounds; M3A passed 30/30,
  M3B passed 13/13, M8 assets/materials passed 76/76, stale-handle proof passed
  7/7, and M8 visual proof passed 2/2.
- `scoped`: `CLI-MACHINE-OUTPUT` and `ARCH-M3A-SCENE-IMPORT` reject raw machine
  printing, hand-authored JSON in both top-level binaries and the shared output
  module, and unchecked production glTF parsing. Their mutation tests passed;
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, and `doctor --full` passed remotely.
- `full`: canonical source was `/home/johannes/projects/scena`, branch `main`,
  HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`. The manually bootstrapped
  isolated copy `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`
  used `/home/johannes/.cache/codex-targets/scena-c01-input`; canonical and
  remote `AGENTS.md` hashes matched, and the complete skill manifests matched.
  All Scena package tests passed with `SCENA_RELEASE_COMMIT` pinned to that HEAD
  because the isolated copy intentionally has no `.git`. The complete xtask
  suite then passed 278/278 with that override removed, preserving synthetic
  commit-mismatch tests. This environment split is required by both fail-closed
  provenance contracts and is not a product failure.
- `skipped`: a real Chrome browser run did not execute because the test tool
  downloaded ChromeDriver 151 and received HTTP 404 against the installed
  browser before being killed. The Node WASM runtime proof is green; this
  browser-toolchain limitation is recorded separately and no browser success is
  inferred from it. No commit, push, tag, merge, release, or publish action was
  taken.

### C02 — Make import create/replace/remove transactional

Scope: B2, N02; prerequisite for F13 watch mode.

Test first:

- [x] Repeatedly replace an import and assert constant node/root/draw counts,
  absence of old roots, and stale rejection for old handles.
- [x] Inject late failures at child linking, anchor creation, connector
  creation, and skin resolution; snapshot nodes, imports, revisions, renderer
  output, and old-import liveness before/after.
- [x] Assert every failed create/replace leaves the original live and visible,
  with no partial replacement nodes or cache/handle registrations.

Implementation:

- [x] Prevalidate the complete `SceneAsset` graph and all late-fallible
  references before inserting nodes; reject cycles/DAG violations explicitly.
- [x] Instantiate into a detached transaction/journal or temporary scene-owned
  structure.
- [x] Commit the new ownership graph atomically, then remove all old owned roots
  and invalidate old handles/import lifecycle in one revision boundary.
- [x] Roll back node, overlay, animation, handle, anchor, connector, skin, and
  revision mutations on every error.
- [x] Define whether replacement preserves user-authored runtime overrides;
  test the chosen policy.
- [x] Pin the ownership/lifecycle matrix in doctor.

Done when:

- [x] Success produces exactly one live import graph.
- [x] Failure is observably a no-op other than a returned diagnostic.
- [x] Hot reload remains bounded over a long replacement loop.

C02 validation ledger (2026-07-16):

- `focused`: the initial three transaction regressions failed against the old
  behavior: repeated replacement retained the old root; an injected child-link
  failure grew nodes from 5 to 7, root children from 3 to 4, and the structure
  revision from 4 to 6; and a failed replacement made the original import stale.
  The final focused suite passes 8/8. Its 256-replacement loop proves constant
  live node/root/draw counts, absence of every retired root, byte-identical
  rendered pixels, stale old imports, and one live replacement graph.
- `focused`: child-link, anchor, connector, and skin-reference failures now
  compare complete scene-state snapshots and rendered output byte-for-byte.
  Failed create and failed replace leave all node/resource registries, import
  liveness, revision lanes, and pixels unchanged. Separate tests reject cycles,
  multiple parents, foreign imports, missing replacement roots, and multi-root
  removal without partial mutation.
- `focused`: replacement intentionally starts from fresh asset-authored runtime
  state rather than preserving user overrides; the policy test also pins one
  revision boundary. Import-owned anchor/connector handles remain typed stale
  after removal, while direct non-import deletion remains `Missing`. A 64-cycle
  regression proves those compatibility tombstones stay bounded to the latest
  generation per slot and that the live registries return to zero.
- `scoped`: prevalidation now covers child indices, multiple parents, cycles,
  anchor/connector metadata, and skin/joint references. `SceneTransaction`
  snapshots every current scene CPU registry and revision lane, rolls back on
  drop, and collapses a successful create/replace/remove to one commit boundary.
  Replacement commits the new graph and removes old subtrees before staling the
  old lifecycle flag. The M7 stale-connector regression passed, Phase 4 native
  primitives passed 9/9, M3A passed 30/30, and the M8 stale-handle suite passed
  7/7.
- `scoped`: `ARCH-M3A-SCENE-IMPORT` pins the transaction snapshot, complete
  prevalidation, bounded retirement registries, all eight focused tests, and
  ordered `transaction -> instantiate -> remove -> commit -> stale -> return`
  replacement lifecycle. Its mutation fixture first evaded the prior literal
  guard and was accepted; the strengthened ordered checker now rejects the
  premature-stale mutation. The complete xtask suite passed 279/279,
  `doctor --full` passed, `cargo fmt --all --check` passed, and `cargo clippy
  --workspace --all-targets -- -D warnings` passed remotely.
- `full`: canonical source remained `/home/johannes/projects/scena`, branch
  `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`. The manually
  bootstrapped isolated copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence` used
  `/home/johannes/.cache/codex-targets/scena-c01-input`; canonical and remote
  `AGENTS.md` plus complete skill manifests matched before validation. The final
  `cargo test -p scena` run exited 0 with the source commit explicitly pinned,
  including 251 library tests, every integration suite, and 58 passing plus 4
  compile-fail doctests. All-feature rustdoc with warnings denied also passed.
- `skipped`: no real-browser or hardware-GPU lane was needed for this
  backend-neutral scene-ownership transaction. The focused test includes
  deterministic CPU rendered-output equality, and the broad package run covers
  available headless GPU integrations; no claim of real hardware-GPU proof is
  made. No commit, push, tag, merge, release, or publish action was taken.

### C03 — Make texture identity/decoding fail-safe and spec-correct

Scope: B3, B15 regression lock, N04; prerequisite for F9 WebP extension work.

Test first:

- [x] Load two GLB assets sharing embedded image index 0 and sampler/slot but
  containing distinct pixels; assert distinct cache identity, decoded content,
  provenance, and rendered color.
- [x] Repeat for embedded Basis/KTX2 fallback and prove same-asset dedup still
  works.
- [x] Load a real native WebP fixture and assert decoded dimensions/pixels and
  rendered texture output; if native WebP is intentionally unsupported, assert
  a fail-closed error before a texture handle is accepted.
- [x] Keep negative KTX2 DFD/slot mismatch fixtures and add compliant color and
  non-color positive fixtures.

Implementation:

- [x] Namespace embedded-image cache keys by stable source/asset identity or
  byte digest, not only image index.
- [x] Make cache provenance immutable and consistent with decoded pixels;
  never replace source bytes while retaining an older decode.
- [x] Add a maintained native WebP decoder behind a documented feature/default
  policy, or remove native support claims and reject WebP structurally.
- [x] Keep the KHR_texture_basisu mismatch gate. Improve the diagnostic with
  slot role, DFD transfer/color primaries, expected value, and repair help.
- [x] Add doctor coverage aligning advertised formats, feature flags, actual
  decode arms, tests, and docs.

C03 validation ledger (2026-07-16):

- `focused`: before implementation, the default C03 suite failed 0/3 because
  distinct embedded GLB and Basis-fallback images reused one texture handle and
  native WebP produced no decoded dimensions; the `ktx2` suite passed only the
  compliant-positive row and failed 4/5 on the same collision, missing WebP
  pixels, and an incomplete mismatch diagnostic. The doctor mutation initially
  failed 0/1 because replacing `memory:image-sha256-` with an index identity
  produced no finding. After implementation, default C03 passed 4/4 and the
  `ktx2` configuration passed 6/6, including rendered red/green separation,
  same-asset dedup, a real lossless WebP pixel/render fixture, compliant
  color/non-color DFD pairs, actionable mismatch text, and the compact-error
  regression. That last test first failed at 144 bytes and now keeps
  `AssetError` at or below Clippy's 128-byte large-error threshold.
- `scoped`: M8 asset/material integration passed 76/76 with defaults and 80/80
  with `ktx2`. The `ASSETS-C03` mutation and source-enforcement tests passed;
  the complete xtask suite passed 280/280; `doctor --full`, `cargo fmt --all
  --check`, workspace/all-target/all-feature Clippy with warnings denied,
  warning-denied all-feature rustdoc, and the repository's
  `wasm32-unknown-unknown` cargo-check lane passed. The official
  `KHR_texture_basisu` contract was rechecked: color material textures require
  BT709 + sRGB, while non-color textures require unspecified primaries + linear
  transfer. Plain WebP decoding is documented as baseline behavior while
  `EXT_texture_webp` source rebinding remains explicitly deferred.
- `full`: the final remote `cargo test -p scena` checkpoint passed 251 library
  tests, every integration suite, 58 passing doctests, and 4 compile-fail
  doctests. Canonical source remained `/home/johannes/projects/scena`, branch
  `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`; the manually
  bootstrapped isolated copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence` used
  `/home/johannes/.cache/codex-targets/scena-c01-input`, with canonical and
  remote `AGENTS.md` plus complete skill manifests matching before proof.
- `skipped`: no separate real-browser or hardware-GPU run was required for the
  native decoder/cache-identity correction. Deterministic CPU rendered-output
  assertions measure the changed behavior, the supported WASM lane compiles,
  and existing available headless-GPU asset tests passed; no browser KTX2 or
  `EXT_texture_webp` release claim is made. No commit, push, tag, merge,
  release, publish, or issue mutation was performed.

### C04 — Repair glTF quantization, morph, animation, and skin contracts

Scope: B4-B6, B13, B20.

Test first:

- [x] Add signed/unsigned non-normalized quantized POSITION fixtures with node
  dequant transforms; verify vertices, bounds, and rendered output.
- [x] Assert unsupported/invalid integer NORMAL encodings return precise errors
  instead of `(0,0,1)` fallback.
- [x] Add a CUBICSPLINE morph fixture with at least two targets; check endpoints,
  midpoint, tangent influence, and visible deformation.
- [x] Add a multi-primitive mesh whose two primitives have distinct morph
  deltas; seek/play and prove both renderable children change.
- [x] Add target 0 normal-only and target 1 position-only fixtures; preserve
  target count/order and weights. Add tangent-delta lighting/normal-map proof.
- [x] Add empty, one-key-at-zero, nonmonotonic, nonfinite, duplicate-time, wrong
  output-width/type imported clips and decide expected results explicitly.
- [x] Add U8/U16 and float skin-weight fixtures covering zero sum, nonfinite,
  negative, exact normalized, and small rounding error.

Implementation:

- [x] Decode every KHR_mesh_quantization POSITION representation allowed by the
  official extension and distinguish absent from unsupported/malformed.
- [x] Chunk CUBICSPLINE weights at target width so each key exposes in/value/out
  vectors; pass rebound clips through fallible validation.
- [x] Represent source-node binding as one transform target plus zero or more
  renderable morph targets; fan out weight channels only.
- [x] Preserve morph target cardinality. Zero-fill omitted allowed semantics and
  add optional normal/tangent deltas through prepare/render ordering.
- [x] Define a glTF-specific static clip policy rather than blindly applying an
  authored-clip duration rule; reject malformed empty/channel shapes.
- [x] Validate weights finite/nonnegative/nonzero and renormalize valid vectors
  within a documented tolerance.
- [x] Add extension/fixture matrix coverage to doctor.

C04 validation ledger (2026-07-16):

- `focused`: the new seven-test deformation contract started red at compile
  time because tangent morph deltas and `morphed_tangents` did not exist. After
  introducing the narrow API, only 1/7 passed; the remaining failures pinned
  valid non-normalized quantized positions, structural NORMAL errors,
  CUBICSPLINE weight layout, multi-primitive fan-out, morph cardinality,
  imported-clip validation, and skin-vector normalization. Iteration reached
  4/7, then 6/7, then 7/7. Extending the visual tangent proof initially
  produced identical normal-mapped frames and exposed an additional CPU path
  defect: sampled tangent-space normals were being treated as world normals.
  The final TBN conversion makes that proof and all seven focused tests pass.
- `focused`: quantization coverage spans signed/unsigned BYTE/SHORT, normalized
  and non-normalized POSITION accessors, node dequantization transforms,
  vertices, bounds, and deterministic rendered output. Invalid integer NORMAL
  encodings fail with semantic-specific errors. The two-target CUBICSPLINE
  fixture proves endpoints, midpoint, tangent influence, deformation, and
  frame delta. Multi-primitive seek/play changes both children with distinct
  deltas. Sparse normal-only/position-only targets retain order, count, and
  weights; morphed tangents visibly affect normal mapping. Static/malformed
  clips and U8/U16/float skin vectors cover every stated acceptance row.
- `scoped`: M3A passed 30/30 and M3B passed 13/13 after two legacy fixtures
  were corrected from malformed glTF to spec-valid CUBICSPLINE, morph-weight,
  and one-key static channels. Both CPU normal-map pixel tests passed, as did
  the existing headless-GPU tangent-space normal-map proof. M8 then passed
  76/76. The focused doctor mutation rejects the old cubic-weight stride; the
  complete xtask suite passed 281/281 and `doctor --full` reports
  `mode=Full status=pass`.
- `scoped`: doctor caught stale pre-C04 API pins and architecture growth after
  the first implementation pass. Imported validation now has its own 248-line
  owner module; import animation rebinding has its own 48-line owner module;
  the parent files are 471 and 498 physical lines respectively. Current M3B
  and C04 ownership/API contracts are source-enforced, and the release-readiness
  no-open-deferrals test passes.
- `full`: the remote integration checkpoint passed 251 library tests and every
  integration binary. One run stopped only when the builder filesystem became
  full while writing an M2 PPM; after deleting only the task-scoped target
  cache, that exact visual test passed and every remaining integration binary
  passed in one resumed invocation. The final code also passes
  `cargo fmt --all --check`, workspace/all-target/all-feature Clippy with
  warnings denied, warning-denied all-feature rustdoc, and 58 passing plus 4
  compile-fail doctests. The official `KHR_mesh_quantization` extension and
  glTF animation/accessor contracts were used as the semantic reference.
- `full`: the supported CI command `cargo check --target
  wasm32-unknown-unknown` passes. The broader release-script command with
  `--all-features` fails before compiling `scena` because feature `icc` pulls
  native `lcms2` C sources into the freestanding WASM target. That independent
  feature-composition/release-gate defect is recorded under Q07; it is not
  hidden as C04 evidence.
- `skipped`: no separate real-browser or hardware-GPU run was required for the
  parser/CPU-prepare correction. Deterministic rendered-output tests cover the
  changed behavior, the supported WASM lane compiles, and the available
  headless-GPU tangent-space proof passes; no real-hardware GPU claim is made.
  No commit, push, tag, merge, release, publish, or issue mutation was
  performed.

### C05 — Fix unit conversion exactly once; lock correct marker semantics

Scope: refuted B11, N03.

Test first:

- [x] Add two- and three-level imports in millimeters, centimeters, inches, and
  feet with authored non-unit scales; assert world translations, geometry
  bounds, and scale exactly once.
- [x] Animate scale on a non-meter import and prove animation values remain
  dimensionless.
- [x] Add nested anchors/connectors with explicit and inherited units; assert
  world alignment and compatibility while retaining source-unit locals.
- [x] Add a regression test proving “convert marker locals to meters” would
  double-convert and is rejected by expectations.

Implementation:

- [x] Choose one unit boundary: synthetic import wrapper/root, or explicit
  geometry/translation conversion. Do not multiply the unit factor into every
  descendant's local scale.
- [x] Never unit-convert animation `Scale` values.
- [x] Preserve anchor/connector authoring units and compose them through the
  chosen import transform exactly once.
- [x] Document local versus world unit semantics in public API/assets docs.

C05 validation ledger (2026-07-16):

- `focused`: the new four-test unit contract first failed 0/4 against the old
  importer. The observed values pinned each defect rather than merely checking
  for a generic mismatch: a millimeter marker resolved at `0.100075` instead of
  `0.175`, an explicit-unit marker at `0.10105` instead of `1.15`, the import
  exposed no distinct unit root, and an authored scale key became `0.002`
  instead of remaining `2.0`. After the implementation, all 4/4 tests pass for
  millimeter, centimeter, inch, and foot imports; two- and three-level node
  hierarchies; non-unit authored scales; world translations/scales/bounds;
  animated scale; inherited markers; explicit-unit markers; and connector
  placement.
- `focused`: non-meter imports now receive one synthetic Empty placement root
  scaled by `SourceUnits::meters_per_unit()`. Source-node translations, scales,
  instance transforms, and animation values retain source-local semantics;
  coordinate-system conversion remains per value. Meter imports retain their
  original root shape. `SceneImport::roots()` owns the placement root so remove
  and replace operations remain import-atomic. The B11 regression lock proves
  explicit marker units are converted into import-local units, not meters, and
  therefore are not converted a second time by the root.
- `scoped`: the stale M3 expectations were updated to assert both preserved
  source-local values and unchanged meter-space world values. M3A passed 30/30,
  M3B passed 13/13, and the complete M7 ergonomics/connector suite passed
  88/88. The `ASSETS-C05` doctor rule pins the single conversion boundary,
  source-local animation scale, marker conversion ratio, public documentation,
  and all four focused tests. Its mutation test removes the root conversion and
  is rejected 1/1; the complete xtask suite passes 282/282 and `doctor --full`
  reports `mode=Full status=pass`.
- `scoped`: `cargo fmt --all --check`, workspace/all-target/all-feature Clippy
  with warnings denied, warning-denied all-feature rustdoc, and the supported
  `wasm32-unknown-unknown` cargo-check lane pass on the remote builder. Public
  asset and units/axes documentation now distinguish source-local values,
  dimensionless scale, meter-space world composition, marker metadata, and the
  requirement to preserve the synthetic root's scale when relocating an
  import. The changelog records the user-visible correction.
- `full`: because import-root topology is public behavior, one integration
  checkpoint ran `cargo test -p scena` after the focused and scoped gates. It
  passed 251 library tests, every integration binary (including C05 4/4, M3A
  30/30, M3B 13/13, M7 88/88, and M8 76/76), 58 doctests, and 4 compile-fail
  doctests. Canonical source remained `/home/johannes/projects/scena`, branch
  `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`; the manually
  bootstrapped isolated builder copy was
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence` with target
  cache `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `skipped`: no separate browser or hardware-GPU run was required because unit
  conversion is scene/import transform behavior and the deterministic CPU
  world-transform, bounds, connector-placement, animation, and rendered-output
  suites exercise the changed contract; available headless-GPU tests in the
  full package run also passed. No commit, push, tag, merge, release, publish,
  issue mutation, or external-state mutation was performed.

### C06 — Enforce finite, atomic camera and transform mutation

Scope: B7, B8, B14.

Test first:

- [x] Inject NaN/±infinity through orbit, pan, touch, pinch, direct set,
  batched set, and alignment; assert error/no-op, unchanged state/revisions,
  preserved transitions, and recovery on the next finite event.
- [x] Assert exact pan direction/sign at yaw 0, ±pi/2, pi and pitched camera
  angles through both Rust and browser host paths.
- [x] Batch a valid node with stale/missing instance roots in both orders and
  assert no transform, revision, or transition mutation.

Implementation:

- [x] Centralize finite `Transform` validation at every public scene mutation
  boundary and retain event-level rejection for fast feedback.
- [x] Compute camera view-right/view-up from yaw/pitch for pan while preserving
  documented drag sign and degeneracy behavior.
- [x] Phase 1: resolve and validate all typed batch entries and clone required
  bindings. Phase 2: cancel/apply only after phase 1 succeeds.
- [x] Preflight every node in an instance binding before its update loop.
- [x] Pin the finite/atomic public-boundary test matrix in doctor.

C06 validation ledger (2026-07-16):

- `focused`: the six-test finite/atomic contract started red 0/6. Non-finite
  pointer movement returned `Orbit`; cardinal pan after yaw remained on world
  X; direct `Scene::set_transform` accepted NaN; the SceneHost camera path
  reported a real action and canceled its transition; host pan repeated the
  fixed-axis defect; and a valid node moved to `x=3` before a stale instance
  root failed. After implementation, all 6/6 pass for NaN, positive infinity,
  and negative infinity across pointer orbit, pointer pan, wheel, touch orbit,
  pinch, direct scene set, scene batch set, world-space alignment, node
  insertion, instance insertion/mutation, browser-host camera control, and host
  transform batches.
- `focused`: rejected events are state-preserving no-ops and direct controls
  recover on the next finite event. A rejected host event preserves an active
  camera transition; after the transition owns/rebuilds controls, a fresh
  finite gesture works. Pan expectations pin horizontal signs at yaw `0`,
  `+pi/2`, `-pi/2`, and `pi`, plus pitched view-up at yaw `pi/2`, in both direct
  `OrbitControls` and `SceneHostCore` paths.
- `focused`: `LookupError::InvalidTransform` now enforces finite translation,
  rotation, and scale at the shared scene boundary used by direct/batched
  transforms, world-space helpers, node insertion, and instance transforms.
  Host batches build a complete resolved-update plan, clone and validate every
  instance-root binding/set/instance, then cancel transitions and apply only
  after phase one succeeds. Stale and never-existing instance-root handles were
  tested before and after the valid node; all four order/error combinations
  preserve node pose, dirty revisions, and the existing eased transition.
- `scoped`: the affected SceneHost integration suite passed 49/49; M7
  ergonomics passed 88/88; orbit-limit tests passed 2/2; transform-gizmo tests
  passed 9/9; and the complete library tier passed 251/251. The `SCENE-C06`
  doctor mutation disables the pointer finite guard and is rejected 1/1. The
  complete xtask suite passes 283/283 and live `doctor --full` reports
  `mode=Full status=pass`.
- `scoped`: `cargo fmt --all --check`, workspace/all-target/all-feature Clippy
  with warnings denied, warning-denied all-feature rustdoc, the supported
  default `wasm32-unknown-unknown` check, and the browser library check with
  `-p scena --lib --features scene-host` all pass. API/error docs state the
  finite invariant, atomic batch behavior, recovery behavior, and camera-space
  pan contract; the changelog records the correction.
- `skipped`: the full all-integration package sweep was intentionally not
  repeated after this slice. The changed risk surface is covered by the focused
  matrix, every SceneHost integration test, M7/control/gizmo suites, the full
  library tier, source-derived doctor, and compile/lint/doc gates; the next
  integrated correctness checkpoint will run the broad chain once. A broader
  `--features scene-host` WASM command also selects the native `scena` CLI and
  fails at four native-only `CaptureRgba8::write_png` call sites; the browser
  library is green and the independent target-composition defect is recorded
  under Q07. No commit, push, tag, merge, release, publish, issue mutation, or
  external-state mutation was performed.

### C07 — Encode handle namespaces, not numeric conventions

Scope: B12.

- [x] Add tests passing every handle kind to every resolver in both directions,
  including first slots, stale/reused slots, high generations, and instance
  thresholds; assert exact wrong-namespace versus stale codes and no mutation.
- [x] Reserve explicit kind/tag bits in the handle representation or replace
  parallel untyped tables with one tagged registry.
- [x] Preserve generation-based stale detection inside each namespace.
- [x] Remove range-based “instance root” classification and generation-base
  conventions as namespace mechanisms.
- [x] Update `docs/errors.md` only after tests prove the namespace guarantee.
- [x] Doctor-pin the resolver matrix and forbid new untagged public handle
  tables.

C07 validation ledger (2026-07-16):

- `focused`: the first-slot reproducer started red 0/1: the first import handle
  numerically equaled the first node handle, `set_transform(import, ...)`
  returned success, and it mutated the scene root. After implementation, the
  public node/import/instance-root/animation contract passes 3/3. Every first
  handle kind is distinct, every wrong node/import/animation resolver returns
  exactly `WrongHandleNamespace`, wrong-kind mutable operations preserve scene
  state, and removed/reused handles return the appropriate stale code without
  reviving their old slot identity.
- `focused`: five internal handle-table tests pass for the complete 4-by-4 kind
  matrix across immutable lookup, mutable lookup, and removal; same-kind
  missing versus stale classification; per-kind slot reuse; maximum encoded
  generations; and generation-exhausted slot retirement. The initial high-
  generation proof exposed an ABA edge in the first implementation, so slots
  now carry an explicit retired state and are never reissued after the final
  generation.
- `implementation`: handles reserve 28 slot bits, 21 generation bits, and an
  explicit four-bit kind tag for node, import, instance root, and animation.
  Every value remains below `2^53`, preserving exact browser/JSON integer
  transport. Instance-root dispatch now checks the decoded kind rather than a
  numeric generation threshold; all generation-base constants and range
  classification are gone. Removing an import also invalidates and frees its
  stale animation handles, so dead mixers are not ticked indefinitely and
  their table slots can be generation-safely reused.
- `scoped`: the SceneHost suite passes 49/49; the C06 atomic regression suite
  passes 6/6; transform-gizmo coverage passes 11/11 in the feature-enabled
  lane; and material-variant handle coverage passes 2/2. The `SCENE-C07`
  doctor mutation collapses the import table back into the node kind and is
  rejected 1/1. The complete xtask suite passes 284/284, and live
  `doctor --full` reports `mode=Full status=pass`.
- `scoped`: `cargo fmt --all --check`, workspace/all-target/all-feature Clippy
  with warnings denied, warning-denied all-feature rustdoc, and
  `cargo check -p scena --lib --target wasm32-unknown-unknown --features
  scene-host` pass on the remote builder. Error, browser, API, and schema docs
  now describe opaque kind tags, exact wrong-namespace/missing/stale semantics,
  compatible instance-root APIs, generation retirement, and the JavaScript
  exact-integer bound. The changelog records the user-visible correction.
- `full`: C07 was the promised integration checkpoint after C06 and changes a
  public host/wire contract, so `cargo test -p scena` ran once after focused
  and scoped proof. It passed 251 library tests, every integration target, 58
  doctests, and 4 compile-fail doctests. Canonical source remained
  `/home/johannes/projects/scena`, branch `main`, HEAD
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`; the manually bootstrapped
  isolated builder copy was
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence` with target
  cache `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `skipped`: no rendered-image or hardware-GPU proof was required because
  handle namespace routing is nonvisual; deterministic state-preservation
  tests and the browser-target library compile directly cover the changed
  contract. No commit, push, tag, merge, release, publish, issue mutation, or
  external-state mutation was performed.

### C08 — Define presentation timeline clip bounds

Scope: B9.

- [x] Add tests for missing end, end beyond duration, start beyond duration,
  zero/static clip policy, once/loop boundaries, exact terminal pose, and no
  repeated `failed[]` entries.
- [x] Resolve the bound clip and duration before applying a timeline segment.
- [x] Validate or clamp the effective `[start,end]` using one documented policy.
- [x] Ensure terminal/loop sampling is stable at floating-point boundaries.
- [x] Return one construction-time diagnostic for an invalid segment instead
  of per-tick failure spam.

C08 validation ledger (2026-07-17):

- `focused`: the expanded seven-test presentation-timeline contract started
  with four expected failures: a segment without `end_seconds` failed instead
  of using clip duration, a future invalid start was not rejected before any
  due segment could mutate state, a repeat segment sampled outside its
  subrange instead of wrapping, and zero-duration/static clips were rejected.
  After implementation all 7/7 pass, covering omitted and overlong ends,
  start-past-duration rejection, zero/static clips, once and repeat endpoint
  semantics, floating-point boundary stability, exact terminal pose, and one
  construction-time diagnostic rather than per-tick failure spam.
- `implementation`: timeline construction resolves every animation binding,
  clip duration, and loop mode before due-segment filtering or patch
  application. Missing ends use duration, explicit overlong ends clamp to
  duration, starts past duration return `InvalidInput`, zero-duration clips
  sample time zero, `Once` holds its inclusive terminal sample, and `Repeat`
  wraps its half-open subrange with scale-aware `f32` tolerance. Non-finite,
  negative, and values too large for `f32` are rejected before mutation.
  `docs/schema-contracts.md` documents that policy and the changelog records
  the correction.
- `scoped`: the SceneHost integration suite passes 49/49. The `SCENE-C08`
  source-derived doctor mutation removes the duration clamp and is rejected
  1/1; the initial command used an incorrect `--exact` filter and selected
  zero tests, so it is explicitly excluded from evidence. The corrected
  mutation command passes 1/1, the complete xtask suite passes 285/285, and
  live `doctor --full` reports `mode=Full status=pass`.
- `scoped`: `cargo fmt --all --check`, workspace/all-target/all-feature Clippy
  with warnings denied, warning-denied all-feature rustdoc, and the
  `wasm32-unknown-unknown` scene-host library check all pass on the isolated
  remote builder. All-feature doctests pass 58 tests with 1 ignored, and all 4
  compile-fail doctests pass.
- `full`: the all-feature package checkpoint first exposed three independent
  stale-proof families rather than being treated as a cosmetic red build: the
  generated meshopt glTF incorrectly put `byteStride` on an index buffer view
  and the KTX2 normal-map proof masked direction through background sampling
  and double-sided lighting; finite-transform diagnostic tests still injected
  invalid transforms into live scenes after C06 made those scenes impossible;
  and CLI/stable-contract goldens retained pre-C07 untagged handles. The glTF
  generator and normal proof were corrected and pass 5/5, the detached-report
  diagnostic proofs pass 5/5, 5/5, and 10/10, and all tagged-handle fixture
  suites pass, including stable contracts 57/57 and the live recipe golden
  1/1.
- `full`: before the one stale CLI golden stopped it, the all-feature package
  run passed 286 library tests with 1 ignored plus every alphabetically prior
  integration target, including the repaired compressed-asset and diagnostic
  suites. Only fixtures changed after that stop. The repaired CLI interaction
  suite then passed 5/5, and every remaining integration target passed in one
  continuation sweep: recipe 91/91, schema 4/4, viewer element 10/10,
  SceneHost 49/49, placement 6/6, scene recipe 61/61, stable contracts 57/57,
  transform gizmo 11/11, transmission 1/1, trust-platform repro 3/3,
  visibility diagnosis 10/10, and visual repair 3/3. This combines the
  unchanged green prefix, focused repaired failure, complete green suffix, and
  separate green doctests without rerunning an unchanged 15-minute prefix.
- `provenance`: canonical source remained `/home/johannes/projects/scena`,
  branch `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`.
  `AGENTS.md` and `.codex/skills/**` were manually verified in the isolated
  builder copy `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`;
  the task target cache was
  `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `skipped`: no browser screenshot or hardware-GPU result is required for the
  timeline-math contract because it changes neither rendered pixels nor a
  browser-only path. The broad all-feature checkpoint did exercise the
  deterministic headless-GPU suites. No commit, push, tag, merge, release,
  publish, issue mutation, or other external-state mutation was performed.

### C09 — Make GPU resource/poll diagnostics truthful

Scope: B10, B19; shares implementation with PF01.

- [x] Inventory every baseline, post, MSAA4/8, depth-color, bloom, DoF, SSR,
  bind-group, buffer, texture, and pipeline owner.
- [x] Add tests for stats deltas and return-to-baseline over enable, resize,
  disable, reprepare, context loss, and destruction.
- [x] Move lazy resource creation to prepare/output-prepare under PF01 so stats
  are complete before render.
- [x] Give each resource owner exact additive creation/destruction accounting;
  do not estimate from a partial baseline.
- [x] Replace wasm `(pending,true)` fabrication with explicit
  automatic/unsupported/submitted/confirmed states.
- [x] Keep native destruction retirement completion-confirmed, but do not make
  browser logical bookkeeping depend on a completion callback: WebGPU/WebGL2
  own in-flight object lifetime and must report non-confirming `Automatic`.
- [x] Add source-lifecycle doctor rules and browser proof for honest status.

C09 validation ledger (2026-07-17; corrected 2026-07-19):

- `focused`: test-first work began with an output-setting contract that failed
  because MSAA/post/depth resources were still created lazily, then with typed
  poll-status and `gpu_textures` compile failures and an exact destruction
  assertion that exposed the partial estimator. The completed focused suite
  passes 4/4. It pins the exact no-post baseline
  `(buffers=9, textures=20, pipelines=4, bind_groups=9,
  shader_modules=6, render_targets=8)`, an MSAA4/post configuration
  `(10,27,11,21,9,19)`, exact pending destruction, CPU `Unsupported` polling,
  resize/context recovery, and either the exact MSAA8 allocation set or its
  structured backend rejection. The existing default-FXAA M1 proof now pins
  its distinct exact tuple `(10,23,7,19,9,18)` instead of a stale range.
- `implementation`: `GpuOutputPlan` is constructed by prepare lifecycle and
  passed to both GPU backends. Native prepare now owns depth/depth-color,
  post-processing, MSAA4/8 pipelines and targets, and overlay depth; wasm
  prepare owns depth-color/post and rejects unsupported multisampling with
  `PrepareError::UnsupportedSampleCount`. Draw paths perform no lazy output
  allocation and report `GpuResourcesNotPrepared` on a plan mismatch.
  `GpuResourceStats` is composed additively from the material, light, shadow,
  environment, transmission, depth, stroke, label, post, readback, and core
  owners; the partial aggregate estimator was deleted. Destruction records
  count every retained buffer, texture, pipeline, and bind group, while render
  targets remain an explicit texture subset.
- `implementation`: public polling reports `DevicePollStatus::{Automatic,
  Unsupported,Submitted,Confirmed}`. Native pending work retires only after a
  confirmed device poll. Browser WebGPU and WebGL2 retire scena's logical
  records as `Automatic`: wgpu's browser WebGPU poll is automatic/no-op and the
  JavaScript implementation retains objects referenced by submitted work;
  WebGL2 uses `GlFenceBehavior::AutoFinish` and GL retains deleted in-flight
  objects. Scena neither waits on `on_submitted_work_done` nor claims browser
  GPU completion. The compatibility `gpu_polled` boolean remains true only for
  `Confirmed`.
- `scoped`: the lifecycle suite passes 4/4, the post-processing suite passes
  4/4, and the `SCENE-C09` doctor mutation passes 1/1 by rejecting both a
  draw-time post allocation and restoration of an aggregate estimator.
  `doctor --full` passes. `cargo fmt --all --check`, workspace/all-target/
  all-feature Clippy with warnings denied, and warning-denied all-feature
  rustdoc all pass on the isolated remote builder. The current browser-probe
  wasm package also builds successfully with `wasm-pack` and the
  `browser-probe` feature.
- `browser/root cause`: exact-source hosted run `29684881823` reproduced WebGPU
  stuck in `Submitted/queue-empty`. Instrumentation proved the callback bridge
  was wired correctly but callbacks for the failing device could arrive tens
  of seconds late; a direct Chromium probe separately proved
  `GPUQueue.onSubmittedWorkDone()` and `mapAsync()` work. The defect was
  therefore scena's callback-dependent logical-retirement contract, not a
  missing browser API or command submission. Focused hosted-style WebGPU and
  WebGL2 M6 runs now both pass with a live heavy-resource phase, exact
  `0 -> 2 -> 0` logical-handle recovery, zero pending destructions, backend
  modes `automatic-webgpu`/`automatic-webgl2`, and
  `completion_confirmed=false`.
- `integration`: the earlier 38-result WebGL2 run and Rust checkpoint remain
  useful for unaffected coverage, but their callback-transition assertion is
  superseded by the correction above. The current exact tree has passed both
  focused browser backends, the focused lifecycle unit contract, the C09 doctor
  mutation, and both WASM package builds. One final remote-builder checkpoint
  and the exact pushed-SHA GitHub run are retained as the release-level proof;
  no success is inferred from the failed hosted run.
- `provenance`: canonical source remained `/home/johannes/projects/scena`,
  branch `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`.
  `AGENTS.md` and the complete `.codex/skills/**` tree were manually copied and
  hash-verified in the isolated builder copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; the scoped
  target cache was `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `skipped`: the strict WebGL2 lane provides actual software-backed browser GPU
  execution, but this closure does not claim hardware WebGPU or physical-GPU
  native proof; those backend-specific proof obligations remain in PF01/Q06.
  No commit, push, tag, merge, release, publish, issue mutation, or other
  external-state mutation was performed.

### C10 — Enforce overlay ownership closure

Scope: B17.

- [x] Add independent direct-removal tests for line/label children of
  measurements and leader/label children of callouts, with node/world anchors,
  annotations, and SceneHost handles.
- [x] Choose one public contract: reject direct deletion of generated children
  with a structured ownership error, or delete the complete owned overlay
  closure atomically.
- [x] Compute the transitive ownership closure before any node mutation.
- [x] Remove/update registry, annotation, handles, anchors, and siblings in the
  same transaction.
- [x] Add an internal invariant check that every overlay registry entry owns
  live nodes and no generated overlay node is unowned.

C10 validation ledger (2026-07-17):

- `focused`: the new four-test ownership suite first failed all four contracts:
  direct measurement-line removal left its label live, direct callout-leader
  removal left its label live, a sibling SceneHost callout handle remained
  valid, and the measurement label handle could not be closed through the
  expected ownership path. After implementation the suite passes 4/4. Each
  line/label and leader/label child is removed independently; callouts cover
  both node and world anchors, annotation cleanup, and preservation of the
  non-owned target; SceneHost cases prove both generated handles become
  `StaleNodeHandle` regardless of which child initiated removal.
- `implementation`: every generated measurement/callout node is registered in
  a typed `OverlayOwner` map. `node_removal_closure` resolves the ordinary
  subtree and then expands all encountered owners before mutation. One
  `SceneTransaction` removes the complete closure, detaches every removed root,
  updates node storage, overlay registries, the callout annotation anchor, and
  owner metadata, and advances revisions as one observable boundary.
  SceneHost computes that same closure before removal and invalidates every
  corresponding host handle. Explicit `clear_callout` and
  `clear_measurement_overlay` use the same atomic owner cleanup.
- `invariant`: the internal checker reconstructs the exact expected owner map
  from both overlay registries, requires every owned node to be live, rejects
  duplicate/unowned generated-node metadata, and requires every callout to
  retain its annotation anchor. It runs after unchecked storage removal and at
  the public add/clear/remove transaction boundaries. The `SCENE-C10` doctor
  rule pins the owner map, pre-mutation closure expansion, transactional
  cleanup, SceneHost invalidation, tests, and public docs. Its mutation removes
  closure expansion and is rejected 1/1.
- `scoped`: the focused ownership suite passes 4/4; callouts pass 2/2,
  measurement overlays 2/2, native removal primitives 9/9, and SceneHost
  integration 49/49. `cargo fmt --all --check` and workspace/all-target/
  all-feature Clippy with warnings denied pass on the isolated remote builder.
  The changed SceneHost path compiles for `wasm32-unknown-unknown` with the
  `scene-host` feature. Live `doctor --full` reports
  `mode=Full status=pass`.
- `scoped`: an attempted all-feature WASM check is explicitly excluded from
  green evidence: the existing `icc` feature enables native `lcms2`, whose C
  build fails for `wasm32-unknown-unknown` before scena code because the target
  has no `stdio.h`. The relevant `scene-host` WASM check is green; the broader
  ICC feature-policy defect remains assigned to Q07 rather than being hidden or
  misattributed to C10.
- `provenance`: canonical source remained `/home/johannes/projects/scena`,
  branch `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`.
  `AGENTS.md` and the complete `.codex/skills/**` tree matched the manually
  bootstrapped isolated builder copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; the scoped
  target cache was `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `skipped`: no rendered-image or hardware-GPU proof is needed because C10
  changes deterministic scene ownership/removal state rather than pixels. The
  full workspace test chain is deferred to the C10-C13 integration checkpoint;
  the directly affected suites, host/WASM compile surface, Clippy, formatting,
  mutation gate, and full doctor are green. No commit, push, tag, merge,
  release, publish, issue mutation, or other external-state mutation was
  performed.

### C11 — Make onboarding and active checklists executable truth

Scope: B21, B22, N15, N19.

- [x] Rewrite getting-started snippets from tested `first_visible_render` logic:
  visible camera/framing, correct async load, correct `frame_import(camera,
  &import)`, explicit prepare/render/capture, and complete error context.
- [x] Replace hard-coded `scena = "1.5"` in both getting-started and README with
  current generated metadata or a version-agnostic `cargo add scena` flow.
- [x] Extract/compile every Rust onboarding snippet in CI with the documented
  features.
- [x] Run a deterministic nonblank rendered-output proof for the first scene and
  GLB snippets; compilation alone does not prove visibility.
- [x] Reconcile both stale regions of
  `next-release-easy-use-and-state-of-the-art.md` for LTC, tiled culling, and
  SSR, including summaries and closeout notes.
- [x] Add reverse status-drift detection under D02.
- [x] Add a metadata/doc gate comparing public dependency examples to the
  workspace package version policy.

C11 validation ledger (2026-07-17):

- `focused`: the new six-test onboarding suite first exposed four independent
  defects: README Rust fences were not compile-gated, README still advertised
  `scena = "1.5"`, the first-scene program omitted the default camera contract,
  and the active release checklist still described shipped Round E work as
  future work. The two deterministic rendered-output probes were already green,
  preventing the documentation rewrite from weakening their visibility oracle.
  After the documentation and gate changes, all 6/6 tests pass.
- `implementation`: README and `docs/getting-started.md` now contain complete
  `rust,no_run` programs derived from the visible-render lifecycle. The first
  scene creates and frames a default camera, prepares, renders, captures, and
  writes the image. The GLB program performs contextual async loading,
  instantiation, `frame_import(camera, &import)`, prepare/render/capture, and
  output. Living dependency examples in README, getting-started, and the
  feature guide now use version-agnostic `cargo add scena` commands.
- `compile proof`: cfg-doctest includes expose every primary README and
  getting-started Rust block to rustdoc without changing the normal public API,
  and the Linux CI lane explicitly runs `cargo test --doc`. The isolated remote
  builder compiled the four onboarding programs while the complete doctest run
  passed 62/62 regular and 4/4 compile-fail doctests.
- `rendered proof`: the first-scene and GLB programs each execute a deterministic
  CPU render and require a nonblank visible result. Both probes pass and wrote
  20,749-byte PPM evidence under
  `target/gate-artifacts/c11-onboarding/`. Their SHA-256 values are
  `5bdb23321a3ac43b1f7db843b1f3ebb68360ea1a82ed45f785cd712ddf91ff7c`
  for `first-scene.ppm` and
  `c085c6e8f9512c97afeefd050c35117b1c7b3cc1b9b7cf14bd3bdf8979a29d1f`
  for `glb-scene.ppm`.
- `status truth`: both stale regions of the active next-release checklist now
  state that tiled culling, LTC area lights, and SSR shipped under their actual
  contracts while KTX2 cubemap work remains deferred. The symmetric D02 reverse
  scanner recognizes the feature aliases and rejects shipped features that are
  relabeled deferred, reopened, or future work in either surrounding direction.
- `doctor`: the `SCENA-C11` rule derives the workspace package version, rejects
  numeric scena dependency examples across the three living onboarding docs,
  requires every primary Rust fence to be `rust,no_run`, pins the CI doctest
  command and lifecycle/status contracts, and pins all six focused tests. Two
  mutations prove stale dependency/CI metadata and lowercase historical
  shipped-feature drift are rejected 2/2. Integrating the rule also exposed and
  repaired an older production-assets doctor pin that still required a numeric
  TOML dependency example.
- `scoped`: `cargo fmt --all --check`, workspace/all-target/all-feature Clippy
  with warnings denied, the six focused tests, the complete doctest lane, and
  live `doctor --full` all pass. The full xtask suite passes 289/289, including
  both new mutation tests.
- `provenance`: canonical source remained `/home/johannes/projects/scena`,
  branch `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`.
  `AGENTS.md` and the complete `.codex/skills/**` tree matched the manually
  bootstrapped isolated builder copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; the scoped
  target cache was `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `skipped`: browser/GPU proof is not required for this CPU-visible onboarding
  and documentation contract. The full workspace release chain remains deferred
  to the C10-C13 integration checkpoint. No commit, push, tag, merge, release,
  publish, issue mutation, or other external-state mutation was performed.

### C12 — Make picking match the rendered pose before accelerating it

Scope: N05 and correctness part of P6.

- [x] Add a morph target that moves the only hittable surface away from its
  base pose; assert base ray misses and deformed ray hits.
- [x] Add a skinned mesh with the same assertion and instances with distinct
  root transforms.
- [x] Define hit distance, normal, and singular/negative/nonuniform-scale
  behavior in public docs.
- [x] Reuse the same deformation evaluation/order as prepare/render rather than
  creating a second approximation.
- [x] Only after correctness is green, implement PF06 local-ray/AABB/BVH work.

C12 correctness validation ledger (2026-07-17; PF06 still open):

- `focused red`: the new four-test pose suite initially passed instance-root
  composition and scale semantics but failed both deformation contracts. The
  morph ray continued to hit the obsolete base pose after its surface moved,
  and the skinned ray did the same after its joint translated the surface. This
  was the expected defect, not a compile-only proxy.
- `implementation`: `GeometryDesc::deformed_vertices` now owns the canonical
  morph-then-skin evaluation order. Ordinary render preparation, shadow
  preparation, and asset-aware picking all call that one evaluator. Picking
  passes the current scene morph weights and resolved skin matrices, while
  instance roots continue to compose node and per-instance transforms in scene
  order. Invalid or missing skin inputs fail through the existing structured
  `LookupError::InvalidSkinBinding`, avoiding a new variant in the public
  exhaustive error enum.
- `public contract`: Rust API docs, `docs/api.md`, the Three.js migration guide,
  and troubleshooting now define normalized-ray world distance, world hit
  position, transformed-winding geometric normals, negative/nonuniform scale,
  and singular transforms whose collapsed triangles are not hittable.
- `focused green`: the pose suite passes 4/4; its morph and skin rays miss the
  base pose and hit the rendered pose, distinct transformed instances resolve
  to the correct `InstanceId`, world distance is measured exactly after a Z
  translation, negative scale reverses winding normal, and singular collapse
  misses. The canonical evaluator unit proof passes 1/1.
- `scoped`: the existing C04 glTF deformation suite passes 7/7 and M3B glTF
  animation/deformation passes 13/13, covering the render path after shared
  evaluation. `cargo fmt --all --check` passes. The `SCENE-C12` doctor rule
  pins the shared evaluator across render, shadow, and picking plus all public
  semantics; its mutation replaces live scene deformation inputs with base-pose
  inputs and is rejected 1/1.
- `doctor integration`: the first post-change `doctor --full` run correctly
  rejected a stale M3B substring pin that still required direct
  `skinned_vertices` use inside render preparation. That pin was updated to
  require the shared `deformed_vertices` contract; its final rerun is recorded
  when this checklist edit is synchronized.
- `provenance`: canonical source remained `/home/johannes/projects/scena`,
  branch `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`.
  `AGENTS.md` and the complete `.codex/skills/**` tree matched the manually
  bootstrapped isolated builder copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; the scoped
  target cache was `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `open by design`: PF06 acceleration is not implemented or checked here. The
  performance section says PF00 measurement truth must land before any
  optimization, so local-ray/AABB/BVH/TLAS work will follow PF00 with measured
  counters and parity evidence. C12 is therefore correctness-complete but not
  fully closed. Full workspace/Clippy validation remains at the C10-C13
  integration checkpoint. No commit, push, tag, merge, release, publish, issue
  mutation, or other external-state mutation was performed.

### C13 — Separate strict GPU construction from preferred fallback

Scope: N14 and Q06.

- [x] Add a failure-injection test for `headless_gpu_with_fetcher` and assert the
  current silent CPU success is no longer possible through a strict API.
- [x] Provide explicitly named contracts such as strict `headless_gpu` and
  opt-in `headless_prefer_gpu` returning a structured fallback report.
- [x] Preserve backend inspection, but do not make callers discover fallback
  only after construction.
- [x] Update examples, capabilities, and release lanes so proof-required paths
  use strict construction.

C13 validation ledger (2026-07-17; C10-C13 integration checkpoint):

- `focused red`: the three-constructor unit contract was written before the
  production change and initially failed to compile because the injectable
  strict/preferred helpers did not exist. A second compatibility lock then
  failed against the first implementation: `SceneHostRecipeBuild` had no
  report accessor and its new public report field broke the existing two-field
  destructuring shape. Both failures measured the intended contracts rather
  than unrelated build noise.
- `implementation`: `SceneHostCore::headless_gpu[_with_fetcher]` now propagates
  adapter/device `BuildError` through `SceneHostError` and can never return a
  CPU renderer. Explicit `headless_prefer_gpu[_with_fetcher]` returns the host
  plus `HeadlessBackendSelectionReport`, including requested/selected backend
  and the original typed GPU error. The high-level viewer uses distinct
  `StrictGpu` and `PreferGpu` policies and exposes the report on the completed
  viewer/first render. Recipe construction has separate CPU, strict-GPU, and
  prefer-GPU policies; report state is stored behind private `SceneHostCore`
  state and exposed by an accessor, preserving the public two-field
  `SceneHostRecipeBuild` shape.
- `proof routing`: CLI recipe rendering, input rendering, depth-of-field proof,
  high-level `with_headless_gpu`, CI, and release workflows all use the strict
  path. `examples/scene_host_contracts.rs`, API/capabilities/headless docs, and
  the changelog distinguish strict proof from an explicitly accepted
  application fallback. Backend inspection remains available, but preferred
  construction supplies selection evidence directly instead of requiring a
  later capability guess.
- `focused green`: the injected constructor suite passes 3/3: strict
  construction propagates injected `NoAdapter`, preferred construction reports
  an injected device failure and selected CPU backend, and the public strict
  constructor never succeeds with `Backend::Headless`. The recipe
  compatibility/accessor lock passes 1/1, the complete recipe contract suite
  passes 61/61, and the viewer API suite passes 10/10. The `SCENE-C13` mutation
  that restores a silent strict fallback is rejected 1/1.
- `doctor`: the rule pins strict propagation, typed report fields and accessors,
  viewer/recipe policy separation, strict CLI/example/workflow routing, and
  forbids the old `or_else` fallback in strict paths plus preferred recipe
  construction in proof-required CLI paths. It also forbids reintroducing the
  public recipe-build report field. Live `doctor --full` reports
  `mode=Full status=pass`; the complete xtask suite passes 291/291.
- `full`: the deferred C10-C13 integration checkpoint passed workspace/
  all-target/all-feature Clippy with warnings denied and
  `cargo test -p scena --all-features`. The latter passed 286 library tests
  with one documented ignored external-review fixture, every binary and
  integration target, 62 doctests with one documented ignored asset example,
  and 4/4 compile-fail doctests. Warning-denied workspace/all-feature rustdoc,
  `cargo fmt --check`, and the relevant `wasm32-unknown-unknown` scene-host
  library check also pass.
- `provenance`: canonical source remained `/home/johannes/projects/scena`,
  branch `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`.
  `AGENTS.md` and the complete `.codex/skills/**` tree matched the manually
  bootstrapped isolated builder copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; the scoped
  target cache was `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `skipped`: the Hetzner machine supplied compile/test and software-adapter
  coverage, not independent hardware-GPU/browser release proof. Required
  WebGPU/WebGL2/native-hardware failure semantics and rendered artifacts remain
  owned by Q01-Q06 and are not inferred from this checkpoint. No commit, push,
  tag, merge, release, publish, issue mutation, or other external-state
  mutation was performed.

## 6. Visual and CI proof remediation

### Q01 — Make the WaterBottle headline a live, bound proof

Scope: S1, N08, N19, D01.

- [x] Add a non-env-gated deterministic 256x256 CPU WaterBottle render to the
  default test lane.
- [x] Compare live output to a committed CPU reference with documented
  full-frame and/or region tolerance; record color-space/orientation rules.
- [x] Add known-bad flattened chrome/wrong material/camera mutation outputs and
  prove the oracle rejects each.
- [x] Keep the macOS live GPU render and accurately describe its current
  nonblack, histogram, and up-to-±35 region checks.
- [x] Either enable a stable approved GPU golden-diff lane or stop claiming the
  GPU lane runs reference diff.
- [x] Correct contributor docs that call RGB Chebyshev comparison DeltaE.
- [x] Bind staged PNG, test result, command, adapter, commit, timestamp, metrics,
  and checksums through D01.

Q01 validation ledger (2026-07-17):

- `focused red`: the live 256x256 CPU test was written before its reference was
  committed and rendered the real Khronos WaterBottle for 226.48 seconds before
  failing solely because `reference_cpu_256.png` was absent. The two initial
  xtask contracts then failed because the headless lane required none of the
  Q01 files/producer command and the typed visual-proof validator ignored the
  CPU proof. A dedicated doctor mutation that removed only the exact CI command
  also failed against the initial empty Q01 rule. These were the intended
  rendering, provenance, and source-drift failures.
- `implementation`: `q01_waterbottle_cpu_reference` is an unconditional native
  integration test. It imports the real asset, instantiates and frames it,
  applies deterministic studio lighting and PBR Neutral tone mapping, and
  renders through `Renderer::headless` at 256x256. The committed reference and
  metadata pin RGBA8, sRGB output, top-to-bottom rows, opaque alpha, RGB
  Chebyshev <=4 for at least 99.5% of pixels, RGB RMSE <=2.0, and zero alpha
  mismatches. The same oracle rejects separately written flattened-chrome,
  wrong-material, and background-only wrong-camera mutation PNGs.
- `live green`: the final exact wrapped producer passed 1/1 in 202.21 seconds.
  The live frame matched all 65,536 pixels (`within_tolerance_fraction=1.0`,
  `rgb_rmse=0`, `max_rgb_chebyshev=0`, zero alpha mismatches). Flattened chrome
  was rejected at RMSE 22.326 and 74.887% within tolerance, wrong material at
  RMSE 78.986 and 74.887%, and wrong camera at RMSE 89.635 and 74.887%.
- `bound evidence`: the actual finalized result has the internal Q01
  WaterBottle CPU reference v1 result schema, status `passed`, exact commit
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`, numeric timestamp,
  `Headless`/`software-rasterizer`, the complete color/orientation contract,
  `rust_test_output_observed=true`, command-record SHA-256
  `b83bb95e0dd83a4633a23c857a03374066e53f80489b4b17bbb2036bfbda6e52`,
  log SHA-256
  `09c8c6d0c4244153bcef2e4fa15a54254e053c60a89d704f2c66005533d669c3`,
  and matching hashes for the live/reference and all mutation PNGs. The live
  and approved reference SHA-256 are both
  `922cc35e0c6420d2b3f8e533891291a9d4f9396697ae366f0b93de3c15973da4`.
- `release path`: both CI and release workflows run the exact test through
  `release_lane_command.sh`. Headless lane finalization verifies the passed
  measured command and literal Rust test summary before adding command/log
  hashes. Release staging decodes the 256x256 RGBA8 PNG, validates every metric,
  mutation, backend, commit, timestamp, result and command binding, and emits
  `visual-proof/waterbottle-cpu.json`. Required-artifact, timestamp, commit, and
  typed-proof manifests include the CPU result, live/mutation PNGs, command
  record, log, and generated visual proof. The canonical staging integration
  test passes with that complete contract.
- `GPU truth`: the existing macOS workflow still invokes the strict live GPU
  WaterBottle test. Contributor/headless docs now state its real oracle: more
  than 5,000 nonblack pixels, color-family histograms, and fixed region checks
  normally at RGB Chebyshev 25 with the measured Apple Paravirtual Metal body
  sample allowed up to 35. No required workflow sets `SCENA_REFERENCE_DIFF`, so
  the optional <=16/95% GPU comparison is explicitly diagnostic and the lane
  does not claim a golden diff. Contributor docs no longer call RGB Chebyshev
  DeltaE.
- `scoped`: Q01 xtask enforcement passes 4/4, including finalized-result tamper
  rejection and the CI-command doctor mutation. The repaired headless-lane and
  canonical staging integration tests pass 1/1 each. `cargo fmt --all --check`,
  xtask all-target Clippy with warnings denied, `doctor --full`, and the full
  xtask suite pass; the final suite result is 295/295.
- `provenance`: canonical source remained `/home/johannes/projects/scena`,
  branch `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`.
  `AGENTS.md` and the complete `.codex/skills/**` tree matched the manually
  bootstrapped isolated builder copy
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; the scoped
  target cache was `/home/johannes/.cache/codex-targets/scena-c01-input`.
- `skipped`: no new renderer production behavior or public API changed, so the
  previous C10-C13 full workspace/Clippy/doc/WASM checkpoint was not repeated.
  The Hetzner builder is not hardware-GPU proof; no fresh macOS Metal run is
  inferred here. The isolated headless lane artifact is incomplete only because
  its unrelated M9 producer command was intentionally not rerun; Q01's actual
  wrapped result and its finalization passed. No commit, push, tag, merge,
  release, publish, issue mutation, or other external-state mutation occurred.

### Q02 — Extend, do not replace, live Round E material proof

Scope: S2.

- [x] Preserve the current live WebGL2 crop/DeltaE/material-specific gate.
- [x] Extract one threshold evaluator shared by live WebGL2, CPU, and WebGPU
  surfaces to prevent metric drift.
- [x] Run the evaluator against live CPU-rendered tiles.
- [x] Add a required WebGPU material lane on an available adapter; keep mobile
  proof separate if the environment cannot make it deterministic.
- [x] Add known-bad flat chrome, isotropic brushed metal, identical neighbors,
  lost clearcoat, missing transmission/refraction, and removed texture variance
  fixtures; each must fail for the intended metric.
- [x] Keep committed PNGs as references/artifacts, not as substitutes for live
  renderer invocation.

Q02 validation ledger (2026-07-17):

- `focused`: the shared evaluator contract passes its good fixture and rejects
  all six required mutations—flat chrome, isotropic brushed steel, identical
  neighbors, lost clearcoat, missing transmission/refraction, and removed
  texture variance—under the synthetic, live CPU, live WebGL2, and live WebGPU
  threshold profiles. The dedicated WebGPU material-only source contract also
  passes. The legacy Cloudflare doctor regression was first red with nine
  findings because it required `status: pass`, legacy whole-tile variance, and
  synthetic `measured` booleans. After moving its source pins to the shared
  evaluator and validating the real direct metrics, the focused Cloudflare
  suite passes 3/3.
- `live CPU`: the exact wrapped Rust producer
  `cargo test --test examples_visual_proof
  q02_live_cpu_round_e_showcase_emits_shared_evaluator_frame -- --exact`
  passes 1/1 in 26.41 seconds, then the exact wrapped Node evaluator writes a
  passed `scena.q02.round_e_cpu_material_proof.v1` result. Its 512x384 live
  frame SHA-256 is
  `9dfbc45e06ae901f190ea56198d6f05997722e5d12047fbe5c9e81d70f3333bb`;
  the frame manifest and result bind commit `bea2a36f5a5e5f5610fa578f1915f137e432281c`
  and current evaluator, producer, provenance, and threshold source hashes.
- `live WebGL2`: the exact wrapped Cloudflare proof command passes against the
  proof harness. `scena.q02.round_e_webgl2_material_proof.v1` records all
  twelve live crops, hard per-material reference DeltaE, material-specific and
  neighbor metrics, an empty error list, current source checksums, and live
  canvas SHA-256
  `587a73aa702a8a6acfc107ece6abb6c953fe7345e3c3ae8c90b0c4322b86b14e`.
  The preserved committed PNGs are reference inputs; the lane cannot pass
  without invoking and hashing the live browser canvas and crops.
- `live WebGPU`: the exact required
  `SCENA_BROWSER_BACKENDS=webgpu ... npm run browser:q02-materials` lane passes.
  `scena.q02.round_e_webgpu_material_proof.v1` records backend `WebGpu`, a
  renderer-owned 512x384 RGBA8 GPU copy, empty errors, current source hashes,
  and live-frame SHA-256
  `297d58636095d1225ebff2a1b0473b67280a1119e2104701f9e171c86a4707de`.
  CI and release workflows require this focused lane before the broader M6
  browser lane; mobile remains separate because no deterministic mobile
  adapter is available in this environment.
- `scoped`: Q02 release-lane, artifact, source-mutation, and tamper tests pass
  4/4. The shared-evaluator and material-only Node contracts pass. Remote
  `cargo fmt --all --check` and `doctor --full` pass. The release lanes require
  exact CPU producer/evaluator, WebGL2 proof, and WebGPU material commands plus
  surface-specific artifacts and provenance; `lane_artifacts.rs` is 594
  significant lines after moving validation to its dedicated owner, below the
  enforced 600-line architecture limit.
- `skipped`: the full generic M6 WebGPU run still times out in its independent
  state-lifecycle destruction confirmation after the Q02 material proof has
  passed. Q02 uses the required isolated material lane so that unrelated M6
  state coverage cannot erase real material evidence; Q06 owns strict
  lifecycle/capability completion. No separate mobile proof or hardware-mobile
  claim is made. No commit, push, tag, merge, release, publish, issue mutation,
  or other external-state mutation was performed.
- `provenance`: canonical source is `/home/johannes/projects/scena`, branch
  `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`. The manually
  bootstrapped isolated builder copy is
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; its
  `AGENTS.md` and complete `.codex/skills/**` content match the canonical
  checkout. Scoped Rust gates used `/tmp/scena-q02-xtask-target` because the
  builder root remains under pressure from an unrelated cache.

### Q03 — Replace visual smoke oracles with feature-specific truth

Scope: S3.

- [x] m3a: assert projected region/component counts for import, selection,
  instances, labels, and readback, plus the existing logical pick.
- [x] m3b: render base and sampled poses; require localized centroid/bounds/pixel
  changes for morph, skin, and animation independently.
- [x] m7: assert connector displacement direction and magnitude, not only frame
  inequality.
- [x] Measurement/callout: independently assess leader/line and label regions
  using the existing line/label quality evaluators.
- [x] Add mutation fixtures removing half the instances, freezing morph,
  collapsing skin, reversing connector movement, and deleting the line.
- [x] Add a doctor rule flagging release-named visual tests whose only frame
  oracle is a nonblack count, with an explicit smoke-test allowlist.

Q03 validation ledger (2026-07-17):

- `m3a focused`: the native headless proof passes 1/1 with one imported
  component in its declared projected region, the existing typed logical pick
  and selected-node state, one retained interaction component, two instance
  components, bounded label ink, and a one-component offscreen readback whose
  bytes exactly equal the renderer-owned surface frame. Removing one of two
  instances leaves a nonblack frame but fails `instance_component_count`.
  Selection state does not currently alter headless pixels, so this proof does
  not fabricate a highlight claim: it binds the logical state to the retained
  projected subject. The equivalent Chrome/WASM proof was red because its
  interaction camera was coplanar with the triangle and produced zero pixels;
  after using the canonical NDC fixture distance it passes 1/1, including the
  Canvas2D write/read roundtrip.
- `m3b focused`: Khronos SimpleSkin, AnimatedMorphCube, and RiggedSimple each
  render a base pose and an independently sampled non-rest pose. The evaluator
  requires a changed centroid or bounds plus a localized changed-pixel region
  intersecting the subject for each feature. The first SimpleSkin midpoint was
  an authored rest pose and correctly failed; sampling its non-rest one-second
  pose makes the native proof pass 1/1. Frozen morph and collapsed skin frames
  remain nonblack but fail `morph_pose_change` and `skin_pose_change`. The
  Chrome/WASM morph proof was independently red with a coplanar camera, then
  passes 1/1 after the same camera correction with visible base/sample frames,
  localized differential truth, and exact Canvas2D roundtrip.
- `m7 and overlays focused`: the current combined remote run passes the M3B,
  M7, and inspection-enabled measurement/callout targets 3/3. M7 isolates the
  red connector source by color, requires rightward displacement greater than
  four and at most 24 pixels with at most three pixels of vertical drift, and
  requires a localized frame delta; reversing before/after remains nonblack
  but fails `connector_direction`. Measurement distance and callout leader
  scenes independently derive projected line and label regions and pass the
  existing line and label quality evaluators. Clearing each line region leaves
  label ink/nonblack output but fails `line_missing_antialiasing` and writes a
  named known-bad artifact.
- `browser`: because `wasm-pack 0.14.0` expands an exact `--test` request to
  `cargo build --tests --test ...` and exhausted the task cache compiling every
  integration target, the passing browser evidence uses the exact single-test
  build `cargo test --target wasm32-unknown-unknown --test <target> --no-run`
  followed by `wasm-bindgen-test-runner` on that emitted WASM artifact. Both
  `m3a_browser_rendered_output` and `m3b_browser_rendered_output` pass 1/1 in
  headless Google Chrome 149.0.7827.102 with matching task-scoped ChromeDriver
  149.0.7827.115. These are browser-executed CPU-frame/Canvas2D proofs; they do
  not claim CPU-to-GPU parity, which remains explicitly owned by Q04.
- `doctor`: the new `Q03-FEATURE-VISUAL-ORACLE` mutation tests were first red
  before the rule existed, then pass 2/2: a release-named proof containing only
  a nonblack oracle is rejected, a feature-specific component oracle is
  accepted, and both CI/release workflows must run the inspection-enabled
  measurement proof. The only explicit smoke allowlist entry is the current M6
  browser parity probe with Q04 ownership and rationale. `doctor --full` first
  exposed four removed stable evidence markers during the proof rewrites; the
  markers now bind the actual fixture IDs/connector contract and the exact
  rerun passes.
- `scoped`: remote `cargo fmt --all --check` passes after the final Rust edits.
  The current focused native batch
  `cargo test --features inspection --test m3b_visual_proof --test
  m7_visual_proof --test measurement_visual_proof` passes 3/3. The earlier
  green M3A native and two doctor mutation tests were not rerun after unrelated
  browser-camera/string-marker edits because their risk surfaces did not
  change; their exact focused evidence is reused under the repository's gate
  discipline. A final post-ledger `doctor --full` is required below before Q03
  handoff.
- `skipped`: no renderer production behavior or public API changed in Q03, so
  the full workspace/Clippy/doc/publish chain is deferred to the integration
  checkpoint. The CPU builder cannot establish hardware WebGL2/WebGPU visual
  correctness, and Q03 makes no such claim. Q04 owns actual CPU-to-live-WebGL2
  parity and explicit workflow activation/consolidation of the dormant browser
  targets. No commit, push, tag, merge, release, publish, issue mutation, or
  other external-state mutation was performed.
- `provenance`: canonical source is `/home/johannes/projects/scena`, branch
  `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`. The manually
  bootstrapped isolated builder copy is
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; its
  `AGENTS.md` and complete `.codex/skills/**` hashes match the canonical
  checkout. Final native and browser builds use task-scoped
  `/tmp/scena-q03-browser-target`; the failed all-tests cache was deleted after
  the documented quota failure, without touching unrelated builder caches.

### Q04 — Establish actual CPU-to-browser renderer parity

Scope: S4, N11.

- [x] Use one identical fixture/camera/settings to render CPU headless and
  attached WebGL2 live in the same proof.
- [x] Normalize row orientation, transfer/color space, alpha, and dimensions,
  then compare with bounded RMSE/SSIM/channel/region metrics.
- [x] Add a deliberately perturbed GPU shader/fixture output in the harness and
  prove parity fails.
- [x] Explicitly run m1/m3a/m3b wasm targets or consolidate their intended
  contracts into active browser targets and retire the misleading target names.
- [x] Require both CPU and GPU frame inputs for any test named `parity`.
- [x] Require renderer-owned GPU readback in release artifacts via D01.

Q04 validation ledger (2026-07-17):

- `focused parity red/green`: the new feature-enabled M6 browser test was first
  red because the WebGL2 probe had no parity payload. It now renders
  `m6-identical-unlit-triangle-v1` at 64x64 with the same scene builder,
  perspective camera at `[0, 0, 2]`, default renderer options, opaque black
  background, exposure, and environment through CPU headless and an attached
  WebGL2 renderer in one invocation. The CPU input is
  `renderer-owned-cpu-frame`; the GPU input is
  `renderer-owned-gpu-copy` read from the renderer's attached WebGL2 canvas,
  with bottom-left WebGL rows flipped before comparison. The current Chrome
  run passes with RMSE `0.0786388411`, SSIM `0.9393638741`, p95 channel delta
  `0`, mean channel delta `2.0486653646`, foreground IoU
  `0.9090909091`, and foreground-region RMSE `0.1214950088`. A rare raster
  edge has max delta `241`, so max delta remains diagnostic while the bounded
  p95/mean/region metrics prevent one backend edge pixel from masquerading as
  a full-frame failure.
- `mutation`: the same evaluator overwrites the GPU frame's center half with
  magenta. The known-bad result remains a valid nonempty frame but is rejected
  by RMSE, SSIM, p95 delta, mean delta, foreground IoU, and region RMSE. Its
  RMSE is `0.4431232917`, SSIM is `0.5494257774`, and mean channel delta is
  `54.765625`; release staging requires `rejected=true` and nonempty mutation
  failure codes.
- `normalization and inputs`: `scena.m6.cpu_webgl2_parity.v1` records top-left
  row origin, sRGB8 transfer, straight opaque alpha, exact dimensions, RGB
  comparison, and sRGB8-luma SSIM. The browser test, Node acceptance producer,
  doctor, and stage validator require both complete 16,384-byte RGBA8 frame
  inputs, positive matching dimensions, zero alpha deviations, bounded metrics,
  and a GPU checksum/dimension match to the headline renderer readback.
- `browser targets`: exact single-target WASM builds followed by the matching
  `wasm-bindgen-test-runner` pass in headless Google Chrome 149.0.7827.102 with
  ChromeDriver 149.0.7827.115: `m1_browser_rendered_output` passes 2/2,
  `m3a_browser_rendered_output` passes 1/1,
  `m3b_browser_rendered_output` passes 1/1, and feature-enabled
  `m6_browser_renderer_parity` passes 4/4. Activating dormant M1 first produced
  two black frames from its coplanar camera; the canonical NDC camera fixed
  visibility. Its remaining stale `[158, 0, 159, 255]` assertion was corrected
  to the existing native/reference contract `[188, 0, 188, 255]` for linear
  alpha blending followed by sRGB encoding. Both CI and release now explicitly
  run m1, m3a, m3b, and `m6 ... --features browser-probe`.
- `release artifact`: the focused end-to-end command uses the current
  feature-enabled WASM package with `SCENA_BROWSER_BACKENDS=webgl2` and a
  reduced `SCENA_BROWSER_WORKFLOWS=model-viewer` workflow set. It passes and
  writes
  `target/gate-artifacts/m6-rust-wasm-renderer-probe.json` in the isolated
  builder copy. The release headline has `status=passed`, commit
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`,
  `pixel_source=renderer-owned-gpu-copy`, renderer checksum
  `ddff4f914dfb2525`, and the full CPU/WebGL2 parity object. The page's duplicate
  WebGPU-only readback branch was exposed by this artifact, removed, and is now
  forbidden by doctor. This builder proof exercises a live attached WebGL2
  browser context through ANGLE/SwiftShader; it is not claimed as discrete
  hardware-GPU evidence.
- `staging and doctor`: the stage-validation mutation test was red when
  `parity` was null, then passes after fail-closed schema, source, dimension,
  RGBA input, normalization, threshold, metric, mutation, and checksum-link
  validation. All seven `tests_20` contract tests and the canonical stage
  aggregation test pass. The `Q04-CPU-WEBGL2-PARITY` doctor fixture was red
  before the rule existed, then passes with weak workflow/frame/fallback
  fixtures rejected and the complete fixture accepted. The final remote
  `cargo fmt --all --check` and `doctor --full` pass; doctor also caught and
  repaired its stale M1 blend-value pin.
- `skipped`: the full workspace/Clippy/doc/publish/browser workflow chain is
  deferred to the Q04-Q07 integration checkpoint under the checklist evidence
  ladder. Q04 changed browser-proof readback and release-gate behavior, so those
  broad gates remain required before any release-ready claim. No commit, push,
  tag, merge, release, publish, issue mutation, or other external-state
  mutation was performed.
- `provenance`: canonical source is `/home/johannes/projects/scena`, branch
  `main`, HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`. The manually
  bootstrapped isolated builder copy is
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; its
  `AGENTS.md` hash `03508eb0...` and complete `.codex/skills/**` aggregate hash
  `d00b024f...` match the canonical checkout. Task-scoped Q04 caches under
  `/tmp/scena-q04-*` were rotated or deleted only to recover quota; no unrelated
  cache or checkout was removed.

### Q05 — Strengthen m2 effect-footprint proof

Scope: S5.

- [x] Add on/off pairs and spatial masks for direct light, receiver shadow, IBL,
  AA, bloom, SSAO, OIT, and clipping.
- [x] Require visible receiver darkening for shadow and material-response change
  for IBL, not only resource stats.
- [x] Use tolerant full-frame/region comparison rather than an exact
  cross-platform hash.
- [x] Either consume the parsed `rgba_hash` under a declared exact mode or
  remove it; fail when reference metadata mode and implementation diverge.
- [x] Add quadrant/effect-footprint corruption fixtures that the current three
  sample locations would miss.

Q05 validation ledger (2026-07-17):

- `focused red`: the first quadrant-corruption test recolored a top quadrant
  outside the former center/left/right samples while preserving their values
  and occupancy; the old oracle accepted it, so the test failed for the
  intended reason. The doctor regression was also red at compile time before
  `check_q05_effect_footprint_contracts` existed. These are the test-first
  baselines; an early module-unqualified doctor command selected zero tests
  and is explicitly excluded from evidence.
- `implementation`: direct light, receiver shadow, IBL, FXAA, bloom, SSAO,
  weighted OIT, and clipping now retain their raw off/on frames plus an
  explicit `PixelMask`, minimum changed-pixel count, mean RGB-delta threshold,
  and directional luma threshold where the effect has a truthful direction.
  The shadow pair uses a real receiver/caster scene and requires more than 30
  channel levels of center darkening. The IBL pair requires a visible PBR
  response in addition to cubemap/prefilter/BRDF resource stats. Weighted OIT
  proves both off/on change and complete-frame insertion-order invariance.
- `reference`: both fixture and reference metadata declare
  `quadrant-mean-rgba-v1`; mode disagreement fails before evaluation. Every
  pixel contributes to one of four tolerant RGBA means and occupancy counts
  (`max_abs_diff=3`, occupancy tolerance 4). The unused `rgba_hash` and retired
  three sample fields were removed. Generated artifact companions identify
  `quadrant-mean-rgba-max-abs-diff-3` and distinguish paired-effect proof from
  the remaining standalone FXAA harness smoke.
- `focused green`: the current exact M2 target passes 4/4 on the remote
  builder: all eight live paired footprints, tolerant current references,
  top-quadrant recoloring rejection, and erasure of each effect's declared
  on-frame mask. Sampled generated companions confirm 32x16 direct-light,
  160x80 receiver-shadow, and 16x16 standalone FXAA dimensions with the
  expected proof class and tolerance.
- `doctor`: the fully qualified `Q05-EFFECT-FOOTPRINTS` mutation test passes
  1/1, rejecting sampled/hash metadata, missing pairs or masks, incomplete
  quadrant references, and missing corruption tests. Existing M2 metadata
  enforcement passes 1/1. Live `doctor --full` reports
  `mode=Full status=pass`, and remote `cargo fmt --all --check` passes.
- `provenance`: canonical source and local destination remained
  `/home/johannes/projects/scena`, branch `main`, HEAD
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`. The isolated remote copy is
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`; its
  manually recopied `AGENTS.md` hash `03508eb0...` and complete skills hash
  `d00b024f...` match canonical. The initial
  `/tmp/scena-q05-effects-target` hit its task quota during `doctor --full` and
  only that 1.5 GiB task cache was deleted. The successful current-source M2
  and doctor gates use
  `/home/johannes/.cache/codex-targets/scena-q05-effects`.
- `skipped`: Q05 changes proof and enforcement code, not renderer production
  behavior or a public API, so Clippy, rustdoc, publish, browser, hardware-GPU,
  and the full release chain are deferred to the final integration checkpoint.
  No commit, push, tag, merge, release, publish, issue mutation, or other
  external-state mutation was performed.

### Q06 — Make required GPU lanes fail directly on missing GPU work

Scope: S7, S8, N14.

- [x] Remove `SCENA_BROWSER_ALLOW_UNAVAILABLE=1` from the required WebGPU job or
  move that behavior to a clearly optional diagnostic job.
- [x] Add explicit required-parity mode; `NoAdapter`, zero renderer output, and
  software/fallback backend must fail the required lane itself.
- [x] Stop doctor from positively requiring the unsafe allow-unavailable flag;
  instead forbid it in required jobs.
- [x] Make `linux-native-vulkan` fail when `host_gpu_available=false` and include
  Linux rendered-output in strict native GPU release evidence.
- [x] Keep CPU fallback only in the separately named headless/diagnostic lane.
- [x] Add xtask fixtures where WebGPU is unavailable and Linux records CPU
  fallback; both required release artifacts must be incomplete/rejected.
- [x] Route proof-required construction through C13's strict API.

Validation ledger (2026-07-17):

- `focused`: test-first red proofs were the missing
  `tests/browser/required_gpu_parity.js` module, the missing
  `required_browser_gpu_parity_passes` release validator, and the missing
  `check_q06_required_gpu_lane_contracts` doctor rule. On the isolated remote
  copy, `cargo test -p xtask q06_ -- --nocapture` now passes all three Q06
  mutation/contract tests. `node tests/browser/required_gpu_parity_test.js`
  passes mutations for `NoAdapter`, zero renderer work/readback, SwiftShader,
  absent adapter identity, and hardware-unproven adapters, plus a positive
  discrete-GPU fixture.
- `focused`: the current WASM package was rebuilt with
  `wasm-pack build --dev --target web --out-dir target/m6-browser-pkg .
  --features browser-probe`. Running the live probe with
  `SCENA_BROWSER_BACKENDS=webgpu SCENA_REQUIRE_PARITY=1
  SCENA_BROWSER_WORKFLOWS=depth-overlap npm run browser:m6` on the CPU builder
  exits 1 with `ADAPTER_HARDWARE_UNPROVEN`; the required lane therefore fails
  at the producer instead of emitting an allowed-unavailable pass.
- `focused`: the exact native target
  `SCENA_REQUIRE_PARITY=1 cargo test --test m9_platform_release
  m9_platform_rendered_output_suite_writes_release_artifacts -- --exact
  --nocapture` passes and records `backend=HeadlessGpu`,
  `host_gpu_available=true`, strict required-parity status, nonblack static
  glTF output, and GPU-path directional/point/spot proof. The builder adapter
  is llvmpipe (`device_type=Cpu`), so this establishes strict Vulkan API/backend
  execution rather than physical-hardware acceleration; it is not represented
  as real-GPU evidence.
- `scoped`: `cargo fmt --all --check` and
  `cargo run -p xtask -- doctor --full` pass in
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence` with
  `CARGO_TARGET_DIR=/home/johannes/.cache/codex-targets/scena-q05-effects`.
  `Q06-REQUIRED-GPU-LANES` forbids diagnostic availability flags in both
  required workflows and pins strict browser/native producers, validators,
  mutation fixtures, C13 construction, and release-lane evidence.
- `full`: deferred to the final multi-slice release checkpoint; Q06 changes
  crossed browser/native release proof, so current focused browser/native
  output and the scoped doctor/fmt gates are retained for that checkpoint.
- `skipped`: no physical-hardware WebGPU success is claimed from the CPU-only
  builder. A required real-GPU lane must produce the positive adapter/readback
  artifact before any hardware-specific release claim can pass.

Validation correction (2026-07-19):

- `root cause`: GitHub Actions run `29681381289` proved that the hosted Ubuntu
  WebGPU job rendered far enough to reach the required-parity evaluator, then
  failed solely with `ADAPTER_HARDWARE_UNPROVEN`. The repository has zero
  registered Actions runners, so requiring hardware identity from
  `runs-on: ubuntu-24.04` made the job structurally impossible rather than
  fail-closed.
- `implementation`: push CI now labels that hosted job
  `software-conformance` and still requires real WebGPU device creation,
  submissions, and nonblank renderer-owned readback. The release workflow
  routes strict `SCENA_REQUIRE_PARITY=1` WebGPU proof to
  `[self-hosted, linux, x64, gpu, scena-gpu]`; the manual hardware workflow
  retains the same fail-closed labels and parity requirements.
- `test-first`: the revised Q06 workflow contract first failed because doctor
  still demanded hardware parity from the hosted job, then passed after doctor
  learned the software-conformance versus hardware-release split.
- `scope`: this correction makes ordinary CI executable without accepting
  software output as physical-GPU release evidence. Publishing remains blocked
  unless a matching hardware runner produces the strict source-bound lane.

### Q07 — Make SSIM and ICC claims real

Scope: S9, N13.

- [x] Add a positive live-render CLI SSIM test against an accepted reference.
- [x] Add camera/material/geometry mutations that fail the same SSIM threshold.
- [x] Decide ICC ownership and scope. Either implement real profile conversion
  using the advertised dependency with rendered-output and metadata proof, or
  remove the feature and all docs/capability claims.
- [x] Add a feature-ownership gate: every advertised Cargo feature must have an
  implementation call site, focused tests, and documentation.
- [x] Make feature composition target-aware: the release-readiness command
  `cargo check --target wasm32-unknown-unknown --all-features` currently pulls
  native `lcms2` C sources through `icc` and fails on missing target `stdio.h`.
  Independently, `--features scene-host` also selects the native `scena` CLI,
  whose WASM build calls the native-only `CaptureRgba8::write_png` in four
  command paths; the browser library itself passes with `-p scena --lib
  --features scene-host`.
  Either provide a WASM-safe ICC backend/contract or exclude the native
  dependency with an explicit unsupported diagnostic, make native CLI targets
  target-aware, then make the scripted all-feature WASM gate pass.
- [x] Replace the ignored out-of-repo Cardine fixture with a committed minimal
  fixture or move it to an explicitly external/manual suite.

Q07 validation ledger (2026-07-17):

- `focused red`: the pre-fix release-readiness command
  `cargo check --target wasm32-unknown-unknown --all-features` selected the
  dependency-only `icc` feature and failed while compiling `lcms2-sys` for
  WASM because the target has no `stdio.h`. The feature-ownership doctor test
  was first run before `check_feature_ownership_contracts` existed and failed
  at compile time with the expected missing-function error. These are the
  test-first baselines. The SSIM and committed-fixture changes are proof-only:
  existing production SSIM/quality behavior required stronger live evidence,
  not a production patch.
- `SSIM focused green`:
  `cargo test --all-features --test scena_cli_recipe
  scena_recipe_render_verify_accepts_live_ssim_reference_and_rejects_scene_mutations
  -- --exact --nocapture` passes 1/1. It renders an accepted 128x128 CLI
  reference, rerenders the same recipe at SSIM >= 0.99, then independently
  measures and rejects camera-eye, material-color, and geometry-size mutations
  below that same 0.99 threshold. Each negative CLI result contains the exact
  `reference_ssim_too_low/live-ssim-reference` quality check and compact reason.
- `ICC/feature ownership`: ICC conversion belongs to `Assets` under the RFC,
  but the advertised flag had no conversion call site, output metadata,
  rendered proof, or WASM-safe contract. The false feature and optional
  `lcms2` dependency were therefore removed from the manifest, README, feature
  docs, and lockfile rather than preserving a dependency-only claim. The
  machine-readable `docs/specs/feature-ownership.json` maps all 15 remaining
  non-default features to an owner, implementation token, active focused test,
  and documentation token. `Q07-FEATURE-OWNERSHIP` rejects missing, stale,
  duplicate, or unproven entries; its unmapped and missing-call-site mutations
  pass 2/2 together with the Q07 false-claim fixture.
- `WASM focused green`: both CI and release workflows now run
  `cargo check --target wasm32-unknown-unknown --all-features`. The exact
  command passes on the remote builder in 54.37 seconds. Filesystem PNG output
  remains callable by CLI targets on WASM but returns the existing structured
  `CapturePngError::Io` with an explicit `unsupported on wasm32` diagnostic and
  guidance to use `to_png_bytes()` or the browser capture API. The regenerated
  local `Cargo.lock` is byte-identical to the passing builder lock
  (`sha256=769c988e5c1c408903314568df2970c3099453a5443d3aacd323074338908f1e`)
  and removes `lcms2`, `lcms2-sys`, and their four now-unreachable helpers.
- `fixture focused green`:
  `cargo test --all-features --lib
  committed_minimal_product_quality_fixture_replaces_external_review_data
  -- --nocapture` passes 1/1. The active committed 32x32 overexposed/flat
  product frame fails both `clipped_highlight_fraction_too_high` and
  `subject_luminance_range_too_low`; the ignored environment-dependent Cardine
  loader, helper structs, and out-of-repo path are gone.
- `scoped`: remote `cargo fmt --all --check` passes. Live
  `cargo run -p xtask -- doctor --full` reports `mode=Full status=pass` after a
  known-bad claim-truth fixture proved that dependency-only ICC claims fail
  closed and the ownership registry was corrected to point at the active
  `public_and_proof_wasm_bundles_are_split` test.
- `provenance`: canonical source and local destination remain
  `/home/johannes/projects/scena`, branch `main`, HEAD
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`. The isolated builder copy is
  `/home/johannes/.cache/codex-worktrees/scena-d01-release-evidence`, using
  `CARGO_TARGET_DIR=/home/johannes/.cache/codex-targets/scena-q05-effects`;
  manually recopied `AGENTS.md` (`03508eb0...`) and complete skills
  (`d00b024f...`) match canonical.
- `skipped`: Q07's focused production-facing WASM check and live rendered CLI
  proof are complete. Clippy, rustdoc, publish dry-run, broader browser/GPU
  lanes, and the full release chain are deferred to the final integration
  checkpoint because the remaining PF/FR/O01 slices still change the same
  worktree. No commit, push, tag, merge, release, publish, issue mutation, or
  other external-state mutation was performed.

## 7. Performance remediation

Do not optimize from the review's unmeasured arithmetic. PF00 must land first,
then each optimization must report before/after distributions on the same
builder/profile and preserve deterministic/visual behavior.

### PF00 — Repair performance measurement truth first

Scope: P12, N20, and performance-proof integrity findings.

- [x] Remove fabricated `allocation_bytes: 0`; measure bytes or mark the metric
  unavailable without a passing claim.
- [x] Align advertised regression threshold with stored baseline policy; do not
  claim 5% while accepting 100%.
- [x] Gate prepare p50/p95 in addition to render p95 where prepare is the hot
  path.
- [x] Record optimized profile, toolchain, CPU/GPU/driver, sample count, warmup,
  commit, command, sidecar/cache state, and confidence/distribution fields.
- [x] Add representative distributions for:
  - one-node transform mutation on CPU and GPU prepare+render;
  - directional/area shadow scaling with intersection counters;
  - 100k-triangle deformed and undeformed pick;
  - qualifying/nonqualifying textured CPU bake;
  - static and deformed tangent generation;
  - animation advance with many channels/keyframes/weights;
  - cold environment bake and sidecar hit;
  - native present-only versus synchronous/async capture;
  - first prepared GPU render after every output setting;
  - many unique transformed nodes for draw-uniform indexing.
- [x] Measure allocation bytes, bytes cloned/copied, locks, ray intersections,
  texture samples, pipeline/shader creations, readbacks, and blocking polls—not
  only wall time and allocation count.
- [x] Add doctor registration/provenance checks for every release performance
  row without trying to enforce runtime speed via substring rules.

### PF01 — Move all render-time resource creation into prepare

Scope: P10 and B10.

- [x] Create an explicit output-settings revision/state.
- [x] Move shader module, pipeline, bind group, and render-target creation for
  post, MSAA, depth-color, bloom, DoF, and SSR into prepare/output-prepare.
- [x] Decide whether a setting mutation makes `render()` return `NotPrepared`
  or requires an explicit `prepare_output`; document one consistent lifecycle.
- [x] Share shader modules/layouts across variants where backend-safe.
- [x] Instrument and assert zero shader, pipeline, texture, buffer, or bind-group
  creation in `render()` after successful prepare.
- [x] Complete C09 resource stats/destruction accounting.
- [x] Add native, WebGPU, and WebGL2 on/off/toggle rendered proof.
- [x] Treat `wgpu::PipelineCache` as a later portability decision, not a
  substitute for lifecycle correctness.

### PF02 — Separate presentation from readback

Scope: P9.

- [x] Define explicit present-only, synchronous capture, and asynchronous
  readback APIs/modes.
- [x] Preserve mandatory headless capture and any auto-exposure dependency on
  frame data without forcing window presentation through it.
- [x] Add counters for copy-to-buffer, map, poll, and wait.
- [x] Prove native surface present-only performs zero readback copies and zero
  blocking maps after prepare.
- [x] Add double-buffered batch capture and measure throughput/latency without
  changing frame ordering or ownership semantics.

PF00/PF01/PF02 implementation ledger (2026-07-17):

- `focused red`: the combined PF00 workload test first failed on the absent
  renderer resize API and then on an uninferred profiled-metrics type; the
  PF01/PF02 lifecycle test failed with the expected missing
  `RenderReadbackMode`, `OutputSettingsChanged`, explicit render-mode API, and
  render resource counters; the async test failed with the expected missing
  batch API and in-flight counters.
- `focused green`: the remote PF00 combined workload contract passes 1/1. The
  exact ordered async test passes 1/1 with `[camera A, camera B, camera A]`
  returning distinct middle pixels, matching first/last pixels, and a measured
  two-readback in-flight peak. The PF00 completion batch extended that same
  combined contract with actual asset-lock deltas, source-cubemap and BRDF
  integration counts, bake-output bytes, and all eight headless-GPU output
  settings; the focused test passes 1/1 in 97.01 seconds.
- `scoped`: remote `tests/c09_gpu_resource_lifecycle.rs` passes 6/6. It proves
  output settings use a distinct revision and `NotPrepared` reason, SSAO/DoF
  depth bind groups are prepare-owned and fully counted, render reports zero
  buffer/texture/pipeline/bind-group/shader creation, present-only reports zero
  copy/map/poll/wait, synchronous capture reports one of each, stale bytes
  cannot become a typed capture, and two prepared readback buffers preserve
  batch ordering. The C09 and PF00 doctor mutation tests each pass 1/1. The
  extended PF00 mutation also proves removal of the real asset-lock increment
  is detected.
- `implementation`: native/headless GPU rendering now exposes automatic,
  present-only, and synchronous modes plus ordered two-buffer batch readback.
  Automatic mode keeps headless capture and managed-auto-exposure input while
  attached native surfaces select present-only. Resource-shaped settings
  advance `output_resources_revision`; render rejects stale output resources.
  The prior unconditional end-of-render device wait and render-time SSAO/DoF
  bind-group allocation are removed. Post effects share texture/depth pipeline
  layouts, and the internal/surface FXAA variants share one shader module;
  resource stats count that deduplication exactly. Exact resource stats include
  both readback buffers and prepared depth-texture bind groups. `Assets` now owns a
  clone-shared monotonic lock-acquisition counter, profiled prepares report the
  exact delta, and the IBL baker returns deterministic source-sample, BRDF
  sample, output-texel, and output-byte metrics. The output-settings producer
  measures prepare plus first present-only render for baseline, FXAA, MSAA4,
  MSAA8, bloom, SSAO, SSR, and depth of field, marking unsupported hardware
  rows honestly.
- `provenance`: canonical source/local destination
  `/home/johannes/projects/scena`, branch `main`, HEAD
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated builder snapshot
  `/home/johannes/.cache/codex-worktrees/scena-pf00`, target cache
  `/home/johannes/.cache/codex-targets/scena-pf00`. Manual `AGENTS.md` and
  `.codex/skills/**` copies match canonical hashes.
- `still open`: PF00's producers and metric families are complete, but the
  representative-distributions checkbox remains open until the attached
  native/WebGPU/WebGL2 hardware rows are collected rather than inferred from
  the headless-GPU lane. PF01 still needs required native/WebGPU/WebGL2 toggle
  renders. PF02 still needs an
  attached-native-surface lane proving present-only zero readback on real
  hardware. Those hardware/browser claims remain fail-closed.
- `skipped`: clippy, full workspace tests, rustdoc, WASM/browser builds, real
  native-surface hardware, publish, and release staging remain deferred to the
  final integration checkpoint. No commit, push, tag, merge, release, publish,
  or issue mutation was performed.

PF00/PF01/PF02 hardware-lane readiness and execution ledger (2026-07-18):

- `local hardware host`: the canonical aarch64 checkout host exposes a
  physical Broadcom V3D 7.1.10.2 integrated GPU through Mesa V3DV 25.0.7
  (`PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU`, Vulkan 1.3.305), alongside the
  separately identified llvmpipe CPU device. `/dev/dri/renderD128` is writable
  by the checkout owner, an active Wayland session and Chromium 148 are
  present, and the pinned Rust 1.93.1, Node 20.20.0, `wasm-pack`, installed
  WASM target, and browser dependencies are available. The owner explicitly
  authorized focused local cargo/browser execution for this physical lane.
- `implementation`: `.github/workflows/hardware-gpu.yml` is a manual,
  fail-closed lane for `[self-hosted, linux, x64, gpu, scena-gpu]`. It requires
  a real display and Vulkan installation, rejects missing/CPU/lavapipe/
  llvmpipe/SwiftShader adapters, runs native and attached-surface proofs, runs
  WebGPU and WebGL2 proof from the same checkout, and optionally collects the
  full optimized PF00 distributions. It uploads only the produced gate
  artifacts. The repository currently has zero registered Actions runners, so
  defining this workflow is readiness evidence, not a completed hardware run.
- `native rendered diagnostic`: the focused remote test
  `pf01_native_gpu_output_toggle_renders_off_on_off_without_lazy_resources`
  passes 1/1 in 15.40 seconds on the CPU builder's explicitly diagnostic
  llvmpipe adapter. It renders nonblank off/on/off frames, observes 2,143
  changed channels between off and FXAA-on, reproduces the off frame exactly,
  changes the prepared resource signature from `[10,20,4,9,6,8]` to
  `[11,23,7,19,9,18]`, returns to baseline, and proves zero render-time GPU
  object creation. The JSON SHA-256 is
  `a0854cce67c36bd91a832df2e31ea663b80c1436c60d0a47e7c4871a6f942f87`;
  off/off-again PPMs share SHA-256
  `a37070b5c72f6b7af76d00938fa31b1c9de2d3c26688002225d078e00f39f4f4`,
  while on is
  `14321d6074e32410eadc829990164262198e977be58d648ee9f56e3c77369d9c`.
  The artifact correctly records `release_evidence=false` and adapter type
  `Cpu`; it does not close PF01.
- `attached native producer`: `examples/native_surface_hardware_proof.rs`
  creates a real `winit` window, constructs `PlatformSurface` from the native
  handle, uses the public assets/mesh authoring path, prepares once, and submits
  `PresentOnly`. It fails on software adapters and on any readback copy, map,
  poll, wait, CPU frame copy, render-time GPU object creation, or prepared
  resource-signature change. Focused remote `cargo check --example
  native_surface_hardware_proof` passes. The CPU builder has no hardware GPU or
  display, so the producer was deliberately not relabeled as PF02 evidence.
- `attached native execution`: strict
  `SCENA_REQUIRE_HARDWARE_GPU=1 WGPU_BACKEND=vulkan WGPU_ADAPTER_NAME=V3D cargo
  run --example native_surface_hardware_proof` constructs the V3D hardware
  adapter and attached surface, then stalls during `prepare_with_assets`.
  Stage instrumentation and a debugger stack localize the stall to Mesa V3DV
  pipeline compilation under `vkCreateGraphicsPipelines`; the GL backend also
  stalls at the same prepare stage. No present-only frame was submitted and no
  PF02 artifact was authored. A temporary demand-driven transmission-pipeline
  experiment merely moved the stall to the ordinary scene pipeline and was
  fully reverted; no experimental production renderer change remains.
- `browser producer`: `tests/browser/pf01_output_toggle.js` builds one
  SceneHost package, runs WebGPU and WebGL2 off/on/off rendered captures,
  requires nonblank and changed on-state pixels, exact off-again pixels,
  distinct prepared on-state resources, baseline resource restoration, and
  unchanged resource shape across each render. Required mode binds each result
  to an attached hardware adapter and rejects software/unproven identities.
  The shared selector permits Chromium or Firefox independently per backend.
- `physical WebGL2 PF01 proof`: Chromium 148 on the V3D host passes the strict
  WebGL2-only diagnostic lane in 22.3 seconds. The adapter is
  `ANGLE (Broadcom, V3D 7.1.10.2, OpenGL ES 3.1 Mesa 25.0.7-2+rpt4)` with type
  `IntegratedGpu`. Off/on/off-again hashes are respectively
  `e405e516052f9288`, `0d5ab93bd3726433`, and `e405e516052f9288`; nonblack
  pixels are 2,011/2,555/2,011; resource shapes are stable across each render
  and restore from `[9,22,6,20,9,18]` to baseline `[8,19,3,7,6,6]`.
  `target/gate-artifacts/pf01-output-toggle/browser/browser-output-toggle.json`
  has SHA-256
  `c22353740d1ef30e036f30db3e05a73d2f53f5b52ed43d9c3fb08e285c3f3eb7`
  and honestly records `complete_backend_set=false` and
  `release_evidence=false`.
- `Windows ANGLE root cause (2026-07-18)`: a focused WebGL2 interception probe
  captured the first failing `drawArraysInstanced` call, its complete WGSL,
  ANGLE-translated HLSL, active interface, framebuffer state, and GL error on
  Microsoft Edge using the Intel Arc Pro D3D11 adapter. The program linked,
  but ANGLE's dynamic pixel-executable specialization failed with HLSL
  `X3511: forced to unroll loop, but unrolling failed` at translated lines
  1719 and 2105. Line 1719 is the runtime-bounded normalization loop over the
  five-element local clipped-vertex array in `area_ltc.wgsl`; line 2105 is the
  outer `MAX_GPU_AREA_LIGHTS` loop around LTC plus the nested 16-sample
  fallback in the WebGL2 material shader. Area-light code was compiled even
  when the scene used zero area lights, so the compiler failure poisoned every
  ordinary material draw. Changing fixtures and sampler specialization did
  not alter either failing loop and was fully removed. The captured diagnostic
  is 1.5 MiB with SHA-256
  `5cf1fcec9626405d9f7b99de3f5fa3c5843e8b4252383c27e274fdbade489549`.
- `Windows ANGLE fix/proof (2026-07-18)`: the local clipped vertices are now
  normalized through five literal indices, and the two supported area lights
  are dispatched through explicit constant-index calls while retaining the
  same LTC and 16-sample contribution body. The fixed WASM has SHA-256
  `bb0e0e938d2ec659811ccdf815da46920421980f14f538edc5dd0803d185db48`.
  Edge on Intel Arc Pro driver `32.0.101.8517` then completed the strict
  hardware WebGL2 PF01 lane without a GL error. Off, bloom-only, FXAA-only,
  both-on, and off-again produced respectively 2,743, 3,447, 2,866, 3,482,
  and 2,743 nonblack pixels with hashes `6f52b7bb8b95b9ab`,
  `1a039b92d6541cad`, `596d677e32484e2f`, `40ec748784927f72`, and
  `6f52b7bb8b95b9ab`. The baseline restored exactly, every enabled state
  changed rendered output, resource signatures stayed stable across each
  render and returned to baseline, and hardware attestation passed with no
  failure codes. This focused artifact correctly records
  `complete_backend_set=false` and `release_evidence=false`; it proves the
  Windows WebGL2 fix but does not substitute for the combined backend set.
- `ANGLE regression lock`: the focused renderer test parses and validates the
  complete WebGL2 material shader with Naga and requires the literal-index
  expansion. The new doctor mutation first failed because neither captured
  loop produced a finding, then passed 1/1 after `ARCH-RENDER-TRUTH` began
  forbidding both dynamic loop signatures and requiring the literal-index
  replacements. The first full-doctor attempt then correctly rejected the
  oversized existing test module after this mutation pushed it over the
  600-significant-line budget; moving the test unchanged into `tests_40.rs`
  restored the architecture split. Remote `cargo fmt --all --check` and
  `cargo run -p xtask -- doctor --full` pass from the isolated
  `scena-hardware-proof` snapshot.
- `WebGPU boundary`: Chromium returns no WebGPU adapter on the V3D host even
  with native-Vulkan and ANGLE/Vulkan flag variants. Playwright Firefox 151
  obtains a WebGPU adapter/device (`maxTextureDimension2D=8192`), but PF01
  renderer-owned color capture does not complete, and FR06 reaches prepare then
  stalls on the first async semantic-buffer map/readback. These are failed
  focused measurements, not unavailable/skip passes and not release evidence.
- `evaluator/doctor hardening`: backend comparison now canonicalizes
  `webgl2`/`web_gl2`; the evaluator regression and browser-selector tests pass.
  `SCENA_WEBGPU_BROWSER`, `SCENA_WEBGL2_BROWSER`, and the diagnostic-only
  `SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS` are registered and documented.
  Required hardware workflow source is mechanically forbidden from setting the
  partial flag. The focused C09 mutation was red before the guard and green
  after it; remote `doctor --full` reports `mode=Full status=pass`.
- `doctor/scoped`: the C09/PF01/PF02, PF00, and FR06 hardware-workflow mutation
  tests pass 1/1 each. Remote `cargo fmt --all --check` and `cargo run -p xtask
  -- doctor --full` pass. Doctor pins the proof producers, required workflow
  labels/commands, strict hardware flags, PF00 producer, and native/browser
  output contracts. Canonical source is `/home/johannes/projects/scena` on
  `main` at `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated validation is
  `/home/johannes/.cache/codex-worktrees/scena-hardware-proof` with target
  `/home/johannes/.cache/codex-targets/scena-hardware-proof`; manual bootstrap
  hashes match the canonical AGENTS and skill hashes recorded above.
- `status at 2026-07-18`: the three checkboxes remained unchecked because the required
  native/WebGPU/WebGL2 set is incomplete. Physical WebGL2 is now proven, but
  native V3D pipeline preparation and Firefox WebGPU readback stall, Chromium
  exposes no WebGPU adapter, GitHub reports zero registered Actions runners,
  and the local hardware workflow is not present on `origin`. A future run on
  another capable hardware runner must be evaluated from produced artifacts;
  workflow source, partial V3D output, llvmpipe output, and SwiftShader output
  are not substitutes. No full workspace suite, clippy, rustdoc, publish,
  release, commit, push, tag, merge, or external mutation was performed in
  this focused hardware batch.
- `complete hardware proof (2026-07-19)`: the one-shot Windows V7 lane passed
  with `status=passed`, `hardware_evidence=true`, and the complete
  WebGPU/WebGL2/native backend set on the physical Intel Arc Pro integrated
  GPU. The aggregate correctly records `release_evidence=false` because the
  received artifacts are not bound to one exact source commit; hardware
  behavior is accepted without being relabeled as release provenance.
  Chromium CDP identifies Intel driver `32.0.101.8517`; the native
  surface artifact identifies Vulkan device `32085`, vendor `32902`, driver
  `101.8517`, and `device_type=IntegratedGpu`. WebGPU phase hashes are
  `2e0d586ede41562e`, `bd489411884b88e4`, `837999dfd19b292d`,
  `36a3f1608a9f78d0`, and restored `2e0d586ede41562e`. WebGL2 hashes are
  `6f52b7bb8b95b9ab`, `1a039b92d6541cad`, `596d677e32484e2f`,
  `40ec748784927f72`, and restored `6f52b7bb8b95b9ab`. Native hashes are
  `c0fc18259341159d`, `2614cfb175a95a85`, `964e2b449c4078dd`,
  `e3410d2e955df455`, and restored `c0fc18259341159d`. Every phase is
  nonblank, every enabled effect changes the baseline, the combined effect is
  distinct from its individual effects, and the before/after prepared-resource
  vectors match across each render.
- `PF02 attached-surface proof`: native `PresentOnly` reports exactly zero
  readback copies, map requests, blocking polls, blocking waits, CPU frame-copy
  bytes, and shader/pipeline/texture/buffer/bind-group creations. Prepared
  resources are identical before and after presentation:
  `[10,20,4,11,6,10]`. The artifact has SHA-256
  `02454a7a5e667fba8c07029d81ff3d88f4a9a626ac098c059c9f382506f9a84e`.
- `artifact integrity`: the automatically uploaded evidence archive has
  SHA-256
  `2a56bf16543cbdb8ee76606ef273a39d97a87b15a8e3ffa37fb17d4ae9797cb4`.
  Its independent summary and all 11 required rendered/JSON artifacts passed
  SHA-256 verification after ingestion under
  `target/gate-artifacts/{windows-complete-hardware-proof,pf01-output-toggle,pf01-pf02-native-surface,fr06-semantic-aov}`.
  The PF01 browser JSON has SHA-256
  `e8c97940f498b4e2803c5da6776852caffd87dc66e1c78106f8f5c21af6af9c6`.
- `focused/scoped`: the complete-artifact validator rejects combined-output
  collapse, nonzero PresentOnly map/readback counters, weak FR06 parity,
  non-release native FR06 evidence, and missing visual artifacts; its Node
  regression passes. The source-derived doctor mutation was red before the
  complete-lane pins and green after them. Remote `cargo fmt --all --check`
  and fresh `cargo run -p xtask -- doctor --full` pass in the bootstrapped
  isolated `scena-windows-final-proof` checkout. Broader Rust release gates
  were not repeated because this final hardware ingestion changed only proof
  harnesses, doctor pins, documentation, and generated evidence; the final
  integration checkpoint remains after PF00 and release staging.
- `PF00 optimized distributions (2026-07-19)`: the bootstrapped isolated
  builder checkout
  `/home/johannes/.cache/codex-worktrees/scena-scena-pf00-final`, with target
  `/home/johannes/.cache/codex-targets/scena-scena-pf00-final`, ran exactly
  `SCENA_RUN_PF00_BENCHMARK=1 cargo test --profile perf-test --test
  m9_platform_release m9_pf00_representative_performance_artifact -- --exact
  --nocapture --test-threads=1`. The one required run passed 1/1 in
  `5294.06s`. All ten workloads contain optimized 100-sample nearest-rank
  distributions plus the registered work/allocation/byte/resource counters.
  CPU and GPU-API timings identify `AMD EPYC-Genoa Processor`, Rust 1.93.1,
  profile `perf-test`, and llvmpipe `LLVM 21.1.8`; llvmpipe timing is not
  presented as physical-GPU speed. The separate Intel Arc proof supplies the
  required native/WebGPU/WebGL2 behavior boundary.
- `PF00 provenance correction`: the completed source run exposed a real
  proof-integrity defect: its raw files said `release_evidence=true` while
  carrying `commit_sha=local-checkout`. Focused tests first failed on the
  absent exact-commit classifier, on the hardware validator still requiring a
  release claim, and on the absent immutable-artifact reclassifier. All three
  now pass. The ten raw measurement files, their timestamps, commit labels,
  source checksums, and SHA-256 values were preserved byte-for-byte. The
  original aggregate is retained with SHA-256
  `184080e4d5b44b662f299cc201aec653ffc069da4de2e893a3e539617f62b5f3`;
  the validated classification aggregate has SHA-256
  `7ca454c1aabf9b25a6f87a327c75fcd8fd0492f9570289e1a98dd595a3fa72fa`,
  `status=measured`, `measurement_evidence=true`,
  `hardware_evidence=true`, and `release_evidence=false`. Future producers
  now separate release-scale measurement completeness from exact source
  provenance before writing each workload.
- `PF10 comparison`: the same optimized builder/profile ran one interleaved
  100-pair disabled/enabled comparison. Dense 128-primitive overlap improved
  p50 by `28.61%` and p95 by `27.50%`, while forced enablement on sparse 128
  primitives cost `15.16%` p50 and `19.81%` p95; the default policy therefore
  remains gated on projected overlap and at least 64 primitives. The artifact
  SHA-256 is
  `9a1313c9c00972fc1a4b3dfe82f3b9eea883d37679f01162ed2448827ff9127b`
  and honestly records `measurement_evidence=true` plus
  `release_evidence=false`.
- `performance doctor`: `PF00-PERFORMANCE-TRUTH` now rejects a workload that
  claims release evidence while the registered current bundle is nonrelease,
  requires measured rows to carry measurement classification and an artifact
  SHA-256, and pins the PF00/PF10 exact-provenance producers. The focused
  mutation was observed red with no finding before the doctor change and
  passes after it.
- `hosted Linux baseline correction (2026-07-19)`: exact-source GitHub run
  `29684881823` measured the larger-industrial lane at frame p95 `56.560595 ms`
  and prepare p95 `1930.651602 ms`; the run failed because the generic fallback
  allowed only `1837.5 ms` prepare time. The fixture now has an explicit
  `linux-native-vulkan` row (`60 ms` frame, `1950 ms` prepare, 5% policy),
  matching the existing macOS/Windows lane-selection design. The focused test
  replays the exact failed measurements and passes only through that Linux row;
  `RELEASE-CI-M9` now requires all three hosted lane rows and its mutation test
  rejects removal of Linux coverage.
- `FR06 hosted-asset correction (2026-07-19)`: the same run rendered WebGL2 but
  failed the strict HTTP gate on the fixture's uncommitted
  `textures/albedo.png`. The glTF now references the existing committed
  WaterBottle base-color PNG. The focused WebGL2 FR06 proof passes with 2,010
  semantic hits, 2,010 finite depth samples, and deterministic repeat output;
  doctor pins the exact URI and rejects either URI drift or removal of the
  referenced file.
- `fixture cross-backend correction (2026-07-19)`: an initial replacement with
  WaterBottle's valid but almost entirely black emissive map made the native
  headless movement fixture blank. The exact all-feature library run failed at
  that assertion, and the focused test reproduced it. Using the committed
  base-color map restored visible pixels, then exposed a stale expectation that
  imported mesh-node transforms must full-prepare. The corrected focused test
  proves the dynamic path has no rejection reason, does not recollect
  primitives or rebuild static GPU resources, increments one retained
  draw-uniform update, and moves the rendered centroid. The focused WebGL2
  FR06 proof also passes with no HTTP failure after the final URI correction.

### PF03 — Remove wholesale prepared-data cloning

Scope: P2.

- [x] Split renderer field borrows so CPU raster buffers can mutably borrow
  independently of immutable prepared lists; remove per-frame list clones
  first.
- [x] Add warm-frame tests/counters proving zero primitive/stroke/label list
  clones after unchanged prepare.
- [x] Then represent prepared geometry through shared model-space buffers plus
  per-node/per-draw data, avoiding per-triangle duplicate matrices.
- [x] Measure allocation bytes and bytes copied on 100k-triangle scenes.
- [x] Preserve deterministic ordering, transparency, picking IDs, and resource
  lifetime.

### PF04 — Resolve immutable assets before inner loops

Scope: P3; prerequisite for PF09.

- [x] Add backward-compatible Arc/snapshot accessors; do not silently break the
  public clone-returning getter without semver process.
- [x] Resolve geometry, material, texture, and sampler snapshots once per
  mesh/material before shading loops.
- [x] Prove zero shared-storage mutex acquisitions inside the per-sample loop.
- [x] Add a 256-entry sRGB-to-linear LUT with exhaustive byte parity against the
  current transfer function.
- [x] Test hot reload/cache replacement so snapshots never retain stale content
  beyond the documented revision boundary.

### PF05 — Cache world transforms and resolved visibility top-down

Scope: P4.

- [x] Compute world transforms and inherited visibility in one O(N) top-down
  pass.
- [x] Key/invalidate on structure, transform, and visibility revisions, plus any
  camera-layer state that affects resolved visibility.
- [x] Test deep trees, reparenting, remove/replace, hide/show inheritance,
  camera-layer changes, and unchanged repeated queries.
- [x] Assert no per-query ancestor `Vec` allocation after cache preparation.
- [x] Make cache ownership live in `scene`; renderer consumes prepared scene
  state rather than owning graph truth.

PF03/PF04/PF05 implementation ledger (2026-07-17):

- `focused red`: the combined remote contract failed to compile on the absent
  warm-render clone counters, Arc snapshot accessors, and scene-cache
  stats/query APIs, pinning all three implementation families before the
  production patch.
- `focused green`: the four remote
  `tests/pf03_pf05_hot_path_contracts.rs` contracts pass on the current diff.
  They prove byte-identical warm CPU output with zero prepared
  primitive/stroke/label clones, pointer-shared descriptor snapshots, a
  qualifying adaptive texture bake with hundreds of samples but fewer than 64
  total prepare locks, and one cache rebuild across 100 repeated deep-tree
  transform/visibility queries. The PF03 storage contract additionally proves
  4,096 triangles share one model-space vertex buffer and one draw transform,
  with zero prepared-list copy bytes. Three focused lib tests pass 3/3, exhaustively
  comparing all 256 LUT entries bit-for-bit, exercising reparent,
  remove/replacement, hide/show, and active-camera layer changes, and proving
  that an old immutable texture snapshot retains its old bytes while a cache
  replacement exposes newly decoded content through a distinct snapshot.
- `implementation`: CPU rendering temporarily moves the prepared owner out of
  `Renderer` so immutable lists and mutable frame buffers borrow independently
  without cloning. Asset slotmaps own `Arc` descriptors; compatibility getters
  still clone descriptors while new snapshot getters share ownership.
  `PreparedMaterialTextures` resolves every texture and sampler once per
  material before subdivision/shading. `Scene` owns a reusable top-down cache
  keyed by structure, transform, visibility, active camera, and camera mask.
  Descriptor cache replacement uses copy-on-write `Arc` ownership: live
  snapshots remain immutable for their documented revision while the next
  lookup observes the replacement without aliasing stale content.
- `PF03 closure implementation` (2026-07-18): retained/draw geometry now owns
  one `Arc<[PreparedModelVertex]>` per geometry snapshot and shares one
  `Arc<PreparedDrawTransform>` per node/draw transform. `Primitive` and
  `PreparedPrimitive` no longer duplicate forward/inverse matrices per
  triangle. Full prepare and GPU resource encoding borrow/chain retained lists
  instead of cloning them; depth accounting iterates primitives and instances.
  Profiled metrics include model-buffer, transform, triangle-reference, and
  list-copy bytes rather than reporting the previous partial copy total.
- `doctor`: the PF03-PF05 source mutation test passes and rejects restoring a
  prepared-list clone or per-triangle matrices; it also pins shared geometry
  and draw-transform ownership, snapshot replacement, lock-free material
  sampling, LUT parity, and the scene-owned revision cache.
- `PF03 release-scale measurement` (remote builder, optimized `perf-test`):
  `m9_pf03_release_scale_prepared_storage_artifact` measured 33,334 triangles
  and 100,002 vertices across 10 samples. It recorded one 6,800,136-byte model
  buffer, one 264-byte draw transform, 800,016 triangle-reference bytes, zero
  prepared-list copy bytes, 120,357,297 p95 allocated bytes, and 44.223 ms p95
  prepare time. The same-host pre-change distribution recorded 329,380,041 p95
  allocated bytes, so allocation fell about 63%. Artifact:
  `/home/johannes/.cache/codex-worktrees/scena-pf03-pf08-close/target/gate-artifacts/pf03/prepared-storage-100k-triangles.json`,
  SHA-256 `d8ca3e544eda94d4b9a733497dc0f00f4a8867223607dc6de645c85b1502f21f`.
- `PF03 preservation proof`: the current-diff focused GPU transform test proves
  three dynamic updates reuse static resources and canonical primitive
  collection while moving rendered pixels. The focused transparent-mesh test
  preserves camera-space back-to-front order. The focused FR06 CPU semantic
  AOV test preserves deterministic occlusion, transparent exclusion, and two
  distinct instance identities. GPU vertex unit tests preserve deterministic
  material/depth/transform batching.
- `skipped`: clippy, full workspace tests, rustdoc, browser/GPU proof, and the
  release chain remain deferred to the final integration checkpoint. No
  external Git or release state was mutated.

### PF06 — Add shared spatial acceleration for picking and shadows

Scope: P1, P6 after C12.

- [x] Correct the misleading picking module claim immediately if BVH does not
  land in the same slice.
- [x] Add mesh AABB rejection and inverse-transform rays into local space once
  per mesh/instance; define singular/nonuniform/negative scale handling.
- [x] Build a deterministic per-geometry BLAS/BVH and a scene/world TLAS or
  equivalent acceleration for shadow occluders.
- [x] Key deformation-aware/refit/rebuild behavior on geometry, morph, skin,
  instance, and transform revisions.
- [x] Preserve exact small-scene hit/shadow parity and deterministic tie-breaks.
- [x] Add synthetic counters proving subquadratic ray-triangle growth over scene
  scaling; do not gate on a guessed 6x speedup.
- [x] Cache per-vertex shadow results only with keys that include deformed world
  position and all relevant light/shadow state.

PF06 validation ledger (2026-07-17):

- `focused red`: the new 8,192-triangle static/deformed picking contract first
  failed to compile because no BVH/cache/bounds counters existed. The
  shared-vertex shadow contract separately failed on the missing prepare-cache
  hit/miss surface. No broader test was used to infer the defects.
- `implementation`: immutable triangle geometry owns a clone-shared
  `OnceLock<Arc<TriangleBvh>>` BLAS. Picking inverse-transforms the normalized
  world ray once per mesh and once per instance level, rejects the model-space
  root bounds, traverses the deterministic BVH, sorts candidates back into
  source triangle order for equal-distance ties, and transforms only exact-test
  candidates back to world space. Zero/non-finite scale fails closed;
  nonuniform and mirrored scale preserve world distance and winding normals.
- `deformation/revisions`: morph/skin results always build from the current
  deformed vertex slice and never touch the static cache. CPU shadow prepare
  builds one world-space BVH from the current deformed, instanced, transformed,
  origin-shifted occluders on every prepare, so no geometry/pose/instance/
  transform revision can reuse stale world bounds.
- `shadow cache`: repeated indexed corners reuse directional/area visibility
  only within one prepare. The key includes exact deformed world-position bits,
  a deterministic signature of the relevant directional/area-light sample
  state, and a deterministic signature of every world-space occluder. The cache
  is created inside `collect_prepared_primitives_profiled` and discarded before
  the next prepare.
- `focused green`: PF06 picking/scaling/cache/tie tests pass 4/4; the exact
  shadow BVH counter test passes; C12 morph, skin, instance composition,
  negative/nonuniform scale, singular scale, distance, and normal tests pass
  5/5; existing small-scene directional/area shadow tests plus the new BVH test
  pass 4/4. The 4,096-spread shadow fixture performs fewer than 128 exact
  triangle tests, and the 8,192-triangle picking fixture stays below one eighth
  of the brute-force count without a guessed wall-clock multiplier.
- `doctor`: `PF06-SHARED-SPATIAL-ACCELERATION` pins geometry cache ownership,
  deterministic BVH construction, local-ray/deformation policy, world shadow
  traversal, prepare-scoped state-complete visibility keys, counters, and the
  focused tests. Its mutation rejects restoring the brute-force triangle loop.
- `skipped`: the batch changes CPU picking/prepare internals but not shaders or
  browser-visible contracts. Clippy, broad cargo tests, rustdoc, browser/GPU,
  publish, and release gates remain deferred to the final integration
  checkpoint; the existing C12 and shadow parity tests were the scoped gates.

### PF07 — Cache tangents without breaking deformation

Scope: P5.

- [x] Cache model-space tangents only for static undeformed geometry.
- [x] Transform/re-orthogonalize under nonuniform scale and handle mirrored
  transforms/handedness.
- [x] Recompute or revision-cache tangents after morph/skin deformation.
- [x] Add parity/rendered proof for nonuniform, mirrored, morph, skin, and
  normal-mapped cases.
- [x] Benchmark static and deformed 100k-vertex workloads separately.

### PF08 — Bound CPU texture-bake expansion

Scope: P7.

- [x] Gate transmission/thickness work on actual transmissive material state
  before texture access.
- [x] Hoist material slot lookup, vertex colors, camera/material constants, and
  other triangle invariants.
- [x] Avoid heap allocation for subdivision factor 1 and reuse scratch for
  enabled subdivision.
- [x] Replace fixed 48x48 expansion with screen/UV-footprint adaptive
  tessellation and a documented hard cap.
- [x] Preserve seams, perspective interpolation, material identity, and CPU/GPU
  comparison through rendered proof.
- [x] Benchmark every qualifying texture-role family separately from
  nonqualifying materials.

PF07/PF08 validation ledger (2026-07-17):

- `focused`: the PF08 contract first failed with the old two-argument fixed-48
  subdivision function, allocation-returning subdivision helper, and missing
  hard cap. The PF07 contract separately failed because neither the shared
  geometry cache nor model-space generation/transform seams existed. After the
  implementation, PF07 passed 3/3 and PF08 passed 2/2 on the isolated remote
  builder target.
- `focused`: the M9 contract-scale tangent row proves a cold static miss, warm
  static hit with zero MikkTSpace regeneration, and continued deformed
  regeneration. The texture-bake row measures all 15 supported texture roles
  separately, bounds emitted triangles to at most 48x48, and keeps the
  nonqualifying path at one triangle with zero texture samples.
- `reused`: C04's rendered normal-map morph proof, M3B's rendered skin proof,
  and the existing CPU/headless-GPU normal-map proofs cover deformed and
  normal-mapped output. PF07 adds exact cached-versus-direct MikkTSpace parity
  under nonuniform scale and a mirrored-handedness contract. No broad visual
  suite was rerun because those rendering surfaces did not change after their
  existing proofs.
- `scoped`: `PF07-PF08-BOUNDED-CPU-PREPARE` pins the cache, deformed bypass,
  transmission gates, reusable scratch, adaptive footprint, hard cap, and
  per-role benchmark surface. Its focused mutation rejects restoration of the
  factor-one allocation/fixed expansion family and removal of the rendered
  PF08 CPU/GPU contract.
- `PF07 release-scale distribution` (remote builder, optimized): 100 samples
  each over 100,002 vertices and 33,334 triangles measured the cached static
  path separately from one-position-morph deformation. Static p50/p95 were
  116.100/123.470 ms with one cache hit and zero MikkTSpace generation;
  deformed p50/p95 were 3,883.691/4,083.943 ms with one required generation per
  sample. Artifact:
  `/home/johannes/.cache/codex-targets/scena-pf03-pf08-close/pf03-before/tangent-generation-static-deformed.json`,
  SHA-256 `5b4afcbfad10f8d89aa04e319bc1742ed052e6f3fad2cf56f5964743264a5413`.
  The artifact records the producer target and source checksums; it was emitted
  by `m9_pf00_representative_performance_artifact` with
  `SCENA_RUN_PF00_BENCHMARK=1` under the optimized benchmark profile. Its
  internal command field honestly remains unavailable because that run did not
  set `SCENA_BENCHMARK_COMMAND`.
- `PF08 rendered proof` (remote builder):
  `pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu`
  renders two depth-skewed, shared-triangle textured panels through both CPU
  adaptive baking and the independently fragment-sampled headless-GPU path.
  Both interiors have full foreground coverage; CPU/GPU RMSE is 0.01164 and
  0.01120, mean channel delta is 1.962 and 1.811, and explicit diagonal probes
  reject black seam gaps while red/blue material identities remain distinct.
  JSON SHA-256 is
  `e556595edad4506724b10c7fc0589f7094cf9295c546532c56f2a240fcc6b6d2`.
- `PF08/Q01 integration correction` (2026-07-18): the initial 16x16,
  16-screen-pixels-per-subdivision rule failed the unconditional Q01 golden
  with only 94.42% of pixels within tolerance and visibly erased the authored
  red logo. A screen-only one-pixel rule restored the logo but still erased a
  small cyan texture feature. The final adaptive factor uses the maximum of
  projected screen footprint and decoded texture-texel footprint, retains a
  documented 48x48 ceiling, and keeps the factor-one allocation-free path.
  Q01 then passed against the original unchanged reference at 99.9664% within
  tolerance and RGB RMSE 0.2009; all flattened-chrome, wrong-material, and
  wrong-camera mutations still fail. The PF08 unit pair, 15-role M9 bounded
  work row, CPU/GPU seam/perspective/material parity proof, and fixed-48 doctor
  mutation pass. Doctor now rejects an actual fixed-48 subdivision call rather
  than the legitimate numeric hard-cap declaration.
- `skipped`: clippy, full tests, docs, browser-native GPU lanes, publish, and
  release gates remain deferred to the final integration checkpoint; no broad
  suite was rerun for this focused closure batch.

### PF09 — Parallelize deterministic, lock-free work

Scope: P8, after PF04.

- [x] Benchmark environment sidecar hit/miss separately before parallelizing.
- [x] Parallelize environment faces/rows and eligible mesh preparation only
  after inputs are immutable and inner-loop storage locks are gone.
- [x] Bin triangles per raster row band rather than rescanning all primitives in
  every band.
- [x] Define serial/WASM fallback and oversubscription controls.
- [x] Compare one-thread and parallel output byte-for-byte or under an explicit
  numeric tolerance; record deterministic ordering.

PF09 deterministic-parallel implementation ledger (2026-07-17):

- `focused red`: the combined PF09 lib filter failed on the absent explicit
  worker-controlled environment bake path and absent retained row-band bin
  type. The first oversubscription proof then exposed that a one-item parallel
  iterator may legally execute on its caller; the proof was corrected to run
  inside an explicit Rayon pool rather than weakening the production guard.
- `implementation`: one shared native worker policy respects
  `RAYON_NUM_THREADS`, caps each renderer operation at eight workers, returns
  one inside a Rayon worker, and compiles to a serial WASM policy. Environment
  GGX faces and BRDF rows write disjoint preallocated slices in parallel and
  reduce only integer work counters. CPU raster scratch retains projected
  triangle row bounds and source-index bins; the immutable projection phase is
  eligible for parallel execution, while deterministic serial bin insertion
  preserves source order for opaque, transparent, and picking-ID identity.
- `measurement`: the pre-existing PF00 producer records cold environment bake
  and matching-sidecar-hit distributions separately. New environment metrics
  record bounded workers/tasks, and render metrics record candidate triangles,
  the former all-triangles-per-band count, and warm bin storage growth.
- `focused green`: `cargo test --lib pf09_ -- --nocapture` passes 3/3 for exact
  one-worker/parallel environment output, bounded/nonnested worker selection,
  and an eight-band 256-triangle fixture with fewer than half the old candidate
  scans, stable source ordering, and zero warm capacity growth. The existing
  CPU serial/parallel framebuffer/depth/linear comparison is expanded to the
  same 256-triangle row-distributed fixture.
- `scoped green`: the expanded exact CPU parity test passes 1/1, the existing
  PF00 combined producer passes 1/1 with distinct cold-bake and sidecar-hit
  distributions plus the new parallel counters, and the warm 1024x768 CPU
  allocation guard passes 1/1. That producer also exposed a stale PF10
  draw-uniform assertion which demanded 120 probes after the indexed path had
  improved to one probe per unique transform; it now enforces an honest 16-32
  near-linear bound. Remote `cargo fmt --all --check` and local
  `git diff --check` pass.
- `doctor`: `PF09-DETERMINISTIC-PARALLEL-WORK` pins worker fallback/control,
  environment face/row parallelism, row-bound preparation, band selection,
  metrics, and cold/sidecar distributions. Its mutation restores a `None`
  band selection and is rejected.
- `skipped`: clippy, full workspace tests, rustdoc, browser/GPU, publish, and
  release gates remain deferred to the final integration checkpoint. PF09 does
  not claim a guessed wall-clock speedup; release distributions remain PF00's
  final performance checkpoint.
- `provenance`: canonical source/local destination
  `/home/johannes/projects/scena`, branch `main`, HEAD
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated builder snapshot
  `/home/johannes/.cache/codex-worktrees/scena-pf09`, target cache
  `/home/johannes/.cache/codex-targets/scena-pf09`. Manually copied
  `AGENTS.md` (`2a6a3f62...`) and complete skills (`93257ec7...`) match the
  canonical checkout; the validation copy intentionally contains no `.git`.

### PF10 — Remove measured assorted hot-path waste

Scope: P11, N06.

- [x] Use shared keyframe segment lookup/`partition_point` and reusable output
  scratch for animation sampling.
- [x] Benchmark CPU occlusion prepass benefit and disable/threshold it when the
  extra depth work exceeds saved draw work.
- [x] Carry matrix invertibility/results instead of recomputing; remove the two
  demonstrably redundant inversions first.
- [x] Reuse supersample/SSR/transmission scratch after warmup and measure actual
  allocations rather than `Vec::new()` syntax.
- [x] Replace O(K² x record-count) instance dedup with a stable hash/index.
- [x] Replace linear draw-uniform lookup with node/group or stable bitwise-key
  interning; prove near-linear scaling for unique transforms.
- [x] Return typed JS render results while retaining JSON compatibility; keep
  the existing typed transform batch path and document it.
- [x] Build source-node to runtime-node maps once during import rebind.
- [x] Digest canonical data URIs rather than retaining multi-megabyte map keys.
- [x] Represent absent optional vertex attributes without allocating full WHITE
  or zero vectors, while keeping shader/CPU defaults identical.

PF10 indexed-data-path implementation ledger (2026-07-17):

- `focused red`: the combined `pf10_` lib filter first failed on the absent
  instance-range index, profiled GPU vertex encoder, source-node index, and
  canonical data-URI helper. The optional-attribute test then failed on the
  absent lazy storage constructor before production geometry changed.
- `focused green`: the same remote filter passes 10/10 in 0.02 seconds. It covers
  65,536-key linear/cubic lookup with at most 17 comparisons, caller-owned
  weight output capacity reuse, 4,096 unique bitwise draw uniforms with
  near-linear probes, zero matrix inversions in GPU vertex encoding, 2,048
  unique/repeated instance ranges with bounded collision comparisons, 16,384
  indexed import-node lookups, a 512 KiB data URI reduced to a bounded digest,
  and absent color/UV storage with identical lazy compatibility defaults. It
  also pins GPU/small-scene occlusion-prepass decisions and proves reusable
  effect scratch reports positive cold capacity bytes but zero warm growth.
- `implementation`: animation sampling uses one shared binary partition lookup
  and reuses the scene's morph-weight allocation. Prepared primitives carry
  both inverse results into GPU encoding. Draw uniforms use an insertion-ordered
  bitwise hash interner across primitives, instances, and strokes; instance
  record ranges use a hash bucket plus exact bitwise collision check. Import
  animation rebinding builds one source-index map. Embedded image cache keys
  are SHA-256 identities. `GeometryDesc` stores absent optional attributes as
  lazy defaults, and glTF prepare/shading reads defaults without materializing
  full WHITE/zero vectors; legacy slice accessors still materialize the same
  values on explicit demand. CPU SSR and transmission share retained RGBA
  scratch, material reflections retain their typed buffer, and GPU supersample
  readback reuses renderer-owned storage.
- `doctor`: remote
  `app::tests_34::pf10_hot_path_doctor_rejects_linear_keyframe_scanning` passes
  1/1 and proves the source-derived rule rejects restoration of the linear
  keyframe scan while pinning every implemented PF10 indexed path.
- `still open`: the CPU occlusion prepass is now skipped for all GPU prepares
  and CPU scenes below 64 primitives, but its benefit distribution still needs
  the final performance checkpoint before that box closes. `renderTyped()` is
  implemented alongside the JSON-string `render()` compatibility method and
  source-pinned by doctor, but its box remains open until the deferred WASM
  compile/browser checkpoint proves the exported object shape.
- `provenance`: canonical source/local destination
  `/home/johannes/projects/scena`, branch `main`, HEAD
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated builder snapshot
  `/home/johannes/.cache/codex-worktrees/scena-pf00`, target cache
  `/home/johannes/.cache/codex-targets/scena-pf00`. Manual `AGENTS.md` and
  `.codex/skills/**` copies match canonical hashes.
- `skipped`: no clippy, full workspace test, rustdoc, browser/GPU lane, or
  release chain was run for this batch; the focused data-structure proofs and
  doctor mutation cover its risk surface. No external Git or release state was
  mutated.

PF10 completion addendum (2026-07-17):

- `focused red`: the explicit CPU-occlusion policy test first failed to compile
  because `Renderer` had no getter/setter. The sparse-scene unit proof then
  failed on the absent projected-overlap gate. The real WASM build exposed
  helper visibilities that were too narrow after module splitting, and the
  browser run caught a stale proof call passing an import handle where a node
  handle was required.
- `implementation`: CPU occlusion remains disabled on every GPU prepare and is
  now eligible on CPU only at 64 or more primitives with overlapping 16x16
  projected tiles. `Renderer::set_cpu_occlusion_culling` provides an explicit
  opt-out and invalidates prepared state without changing pixels. The PF10
  producer warms up and alternates enabled/disabled order to remove the cache
  bias found in its discarded first run. `renderTyped()` retains the existing
  JSON `render()` method and returns the same five fields as a native object.
- `focused green`: the overlap unit proof passes 1/1; enabled/disabled
  pixel-parity and draw-count proof passes 1/1; and the PF10 doctor mutation
  passes 1/1, including rejection when the overlap gate is removed. The
  corrected `perf-test` artifact records 100 samples per mode: dense 128 falls
  from 128 to 21 draws (107 culled) and improves prepare-plus-render by 16.41%
  p50 and 16.84% p95; sparse 128 culls zero and skips the depth prepass, with
  only the bounded projection/tile eligibility cost (1.17% p50, 1.07% p95).
  The below-threshold 32-primitive row culls zero and executes the identical
  path; its 0.21% p50 delta plus noisier p95 records the host variance rather
  than attributing work to a prepass that did not run.
- `browser`: a fresh WASM package built successfully, then the WebGL2
  SceneHost proof passed in Chromium and asserted the exported `renderTyped`
  binding, exact native-object fields/types, and equality with JSON
  compatibility output. The renderer was SwiftShader/low-tier, so this closes
  JavaScript object-shape/runtime compatibility only, not hardware GPU claims.
- `doctor/docs`: `PF10-MEASURED-HOT-PATH-WASTE` pins the threshold, overlap
  gate, public opt-out, alternating benchmark producer, and browser assertion.
  The workload is registered in `performance-evidence.json`, and rendering/API
  docs describe the policy.
- `provenance`: canonical source/local destination
  `/home/johannes/projects/scena`, branch `main`, HEAD
  `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated builder snapshot
  `/home/johannes/.cache/codex-worktrees/scena-scena-fr01-fr04`, target cache
  `/home/johannes/.cache/codex-targets/scena-scena-fr01-fr04`; canonical agent
  files hash-matched before every gate.
- `skipped`: no full workspace tests, clippy, rustdoc, native/WebGPU hardware
  lane, publish, or release chain was run for this focused PF10 closure. No Git,
  GitHub, release, or publish state was mutated.

## 8. Feature roadmap checklists

These items are intentionally after correctness, proof integrity, and lifecycle
performance. Closing a feature box means the proposal was scoped and accepted,
not merely coded.

### FR01 — Field schema and vocabulary discovery (F1, N17)

- [x] Define one authoritative field model for types, required fields, enums,
  ranges, defaults, deprecations, and examples.
- [x] Make `schema get` emit that model without hand-copying validator help.
- [x] Add `scena vocab` for closed renderer/recipe vocabularies with stable
  schema/version and owner links.
- [x] Catalog the CLI help/version v1 contracts and every other public contract,
  or explicitly classify internal proof schemas outside the public catalog.
- [x] Add round-trip/invalid fixtures and doctor drift checks.

### FR02 — Build a recipe without rendering (F2)

- [x] Specify `scena recipe build` as load/validate/build-manifest without
  prepare/render/capture; avoid ambiguous `--dry-run` terminology.
- [x] Emit the existing typed ID-to-handle/skipped/diagnostics manifest.
- [x] Define whether remote asset fetch and policy enforcement occur; expose the
  effective policy in output.
- [x] Prove no renderer/GPU/capture construction through instrumentation.
- [x] Add success, broken asset, policy denial, and stable-schema CLI tests.

### FR03 — Apply placement through persistent recipe identity (F3)

- [x] Define a recipe-ID-based patch contract or emit a complete updated recipe.
- [x] Do not use ephemeral SceneHost handles as the persistent default.
- [x] Preserve formatting/order only if explicitly promised; otherwise emit
  canonical JSON and a semantic change summary.
- [x] Add preview/apply/rebuild equivalence tests and stale-source conflict
  diagnostics.

### FR04 — Honest command schemas and policy discovery (F4)

- [x] Complete C11 first.
- [x] Add `emits` success and error schema sets per command in machine help.
- [x] Document and test polymorphic `asset_doctor.v1` load failures for inspect,
  render, and diagnose.
- [x] Add a machine command exposing effective sandbox roots, URL/file policy,
  limits, and source of each policy value.
- [x] Validate every declared output schema against real success/failure CLI
  fixtures.

FR01/FR03/FR04 remediation ledger (2026-07-17):

- `focused red`: the two discovery tests first rejected the absent `vocab` and
  `policy` commands and absent `command_contracts`. The feature-enabled FR03
  test first rejected unknown `--apply`; after the patch contract existed it
  exposed a second real failure when a complete emitted recipe retained a
  source-relative import URI and could not rebuild from a different directory.
- `implementation`: `scena vocab list|get` publishes owner/version/value rows;
  `scena policy recipe` reports effective URI/network/root/limit policy and the
  source of every value; machine help now declares success/error schema sets
  for every command and dispatch failures emit `scena.cli_error.v1`. Placement
  apply emits `scena.recipe_patch.v1`, binds the update to the source SHA-256,
  identifies the import by persistent recipe ID, reports the transform change,
  and includes canonical JSON with all relative import/font/environment/texture
  URIs rebased so the complete recipe remains buildable at a new path.
- `focused green`: the exact FR01/FR04 CLI discovery tests, CLI schema golden,
  live schema-catalog golden, stable-fixture version check, and
  `FR01-FR04-CONTRACT-DISCOVERY` doctor mutation pass on the remote builder.
  The exact feature-enabled FR03 preview/apply/rebuild/stale-source test passes
  1/1 after URI rebasing.
- `scoped`: `cargo fmt --all --check` and `git diff --check` pass for the batch.
  Public catalog entries and stable fixtures cover `scena.vocab.v1`,
  `scena.recipe_policy.v1`, `scena.cli_error.v1`, and
  `scena.recipe_patch.v1`.
- `open`: FR01 still needs an authoritative field-model source wired into
  `schema get` plus explicit round-trip/invalid coverage. FR04 still needs a
  fixture matrix that executes and validates every declared command output.
  FR02 is deliberately separate because the current host recipe builder owns a
  renderer; an honest no-render builder requires a build-context split, not a
  renamed call to the current headless-renderer path.
- `skipped`: clippy, broad cargo tests, rustdoc, browser/GPU lanes, publish, and
  release gates were not run. This checkpoint touched CLI/schema/recipe patch
  contracts and used their exact tests; broad integration remains deferred to
  the final release checkpoint.

FR01/FR02/FR04 completion addendum (2026-07-17):

- `focused red`: field discovery first failed because `schema get` had no
  `field_model`; the manifest-only environment fixture first succeeded while
  silently skipping its required HDR; and the CLI matrix rejected advertised
  top-level schemas that the commands never emit. The measured fetch counter
  then corrected the stable glTF expectation from one inferred import to three
  real source/external-resource attempts.
- `implementation`: `scena.field_model.v1` now publishes recipe path types,
  requiredness, enums, ranges, defaults, deprecation flags, and examples from
  recipe-owned constants also consumed by root/import/capture, primitive, and
  render validation. `scena recipe build` uses a private manifest-only host,
  resolves imports/textures/environments under effective policy, strictly
  checks required environment sources, and measures every real asset-store
  fetch attempt while keeping renderer, GPU, prepare, render, and capture
  construction at zero. The CLI evidence matrix removed false raw-build and
  asset-doctor declarations and covers every remaining command/schema/outcome
  pair with a real CLI fixture.
- `focused`: renderer-free recipe build passes 5/5; schema/vocabulary/policy
  discovery passes 8/8; the command output matrix passes 6/6; stable fixture
  version, catalog, schema-entry, and recipe-build-result checks pass 4/4; and
  the `FR01-FR04-CONTRACT-DISCOVERY` mutation passes 1/1 on the remote builder.
- `scoped`: `cargo fmt --all --check` and local `git diff --check` pass. Doctor
  now pins `field_model`, `recipe_build_result`, `recipe_patch`, `vocab`,
  `recipe_policy`, and `cli_error` fixtures. Live `doctor --full` no longer
  reports any FR01/FR02/FR04 or stable-fixture finding, but remains red on 22
  separate open goal findings: architecture owner/source-pin drift and module
  size limits scheduled in later batches.
- `provenance`: canonical source `/home/johannes/projects/scena`, branch `main`,
  HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated builder copy
  `/home/johannes/.cache/codex-worktrees/scena-scena-fr01-fr04`, target cache
  `/home/johannes/.cache/codex-targets/scena-scena-fr01-fr04`. Manually copied
  `AGENTS.md` and `.codex/skills/**` match canonical hashes.
- `skipped`: Clippy, rustdoc, broad package tests, browser/GPU proof, publish,
  and release gates were intentionally not repeated for this CLI/schema batch.
  They remain deferred to the next natural integration/release checkpoint. No
  Git, GitHub, release, publish, or other external state was mutated.

Architecture-hygiene integration addendum (2026-07-17; supersedes the earlier
22-finding status above):

- `focused red`: live `doctor --full` reported 22 source-contract/owner/module
  size findings after the FR01/FR02/FR04 batch. Mutation fixtures also exposed
  that positive source pins, performance contracts, and recipe policy scans did
  not follow split Rust child modules, while the singleton scanner treated
  renderer-owned `RefCell`/lock initialization as global state.
- `implementation`: all production modules now stay at or below 500
  significant lines and xtask modules at or below 600 through owner-preserving
  child modules. Positive source contracts and performance/recipe scanners
  follow Rust module trees; negative checks remain exact-file. Architecture
  ownership, render/shadow/DoF pins, and vocabulary documentation now match the
  live split code. The singleton rule still rejects statics but no longer
  reports renderer-owned cells or locks.
- `focused`: five relevant xtask mutation/regression tests pass: the split
  module contract, singleton false-positive regression, PF06 mutation, PF09
  mutation, and the FR01-FR04 manifest mutation.
- `scoped`: remote `cargo check --workspace --all-targets` passes after one
  path/visibility correction cycle. Remote live `cargo run -p xtask -- doctor
  --full` passes with zero findings. Rust formatting was applied remotely and
  local `git diff --check` passed for that snapshot.
- `skipped`: broad runtime tests, clippy, rustdoc, browser/hardware GPU, publish,
  and release gates were intentionally not repeated for structural source
  moves. The final fresh-target doctor and release checkpoint remain open.

### FR05 — Reuse CAD multi-view for general capture (F5)

- [x] Extract reusable three/canonical-view and contact-sheet logic from
  `recipe inspect-cad`; do not build a parallel renderer loop.
- [x] Define front/top/right/isometric camera conventions, up axis, framing,
  labels, order, and contact-sheet metadata.
- [x] Add explicit turntable sampling and clip frame sequences through the
  normal prepare/render lifecycle.
- [x] Record camera/clip/time per output and add deterministic image/GIF/video
  proof where supported.

FR05 completion ledger (2026-07-17):

- `scope`: RFC review kept this as renderer-owned scene/camera/animation/capture
  composition. It adds no simulation, robotics, PLC, physics, or host-domain
  cadence. The public command is `scena recipe capture <recipe.json> --out-dir
  <dir> [--views front,top,right,isometric|none] [--turntable <frames>] [--clip
  <name> --frames <n>] [--gpu] [--max-imports <n>]`; its stable report is
  `scena.capture_sequence_result.v1`.
- `focused red`: `tests/fr05_capture_sequence.rs` first failed with
  `invalid_command` because `recipe capture` did not exist. The path-safety
  unit then failed to compile because `safe_file_label` did not exist, and the
  total-budget unit failed because the default four views plus 360 turntable
  frames were accepted. The contact-sheet dimension unit likewise failed
  because no bounded thumbnail policy existed.
- `implementation`: one `SceneHostCore` is built, framed, and reused. Every
  frame follows `set_camera -> prepare -> render -> capture`; turntables sample
  evenly spaced yaw at 20 degrees pitch, and authored or imported clips sample
  the inclusive `[0,duration]` interval. Default order is front, top, right,
  isometric in a right-handed +Y-up world. The host-orbit top view records its
  honest one-degree pole offset and -Z screen-up. Per-frame JSON records the
  camera, kind, label, PNG/descriptor paths, original payload hash, and
  canonical/turntable/clip/time metadata.
- `reuse`: CAD inspection and general capture now share one checked raw-RGBA
  contact-sheet compositor/PNG writer. CAD also reuses the general
  `SubjectBounds` implementation and canonical top direction/screen-up
  convention. The general command uses the normal SceneHost lifecycle rather
  than creating another renderer loop. This final extraction was
  behavior-preserving, so its closest deterministic proof is the pre-existing
  CAD rendered integration plus the FR05 rendered integration rather than an
  artificial expected-red production change.
- `bounded/fail-closed`: the combined sequence is capped at 360 frames;
  filesystem labels are ASCII-sanitized and capped at 96 characters so an
  imported clip name cannot escape `--out-dir`; contact-sheet thumbnails are
  capped at 192 pixels on the longest edge, use checked dimensions, and retain
  original-frame hashes in report tiles. Full-resolution numbered PNGs and
  descriptors remain unchanged.
- `public contract`: machine help and its per-command success/error families,
  schema catalog, `schema get` fixture map, catalog/CLI goldens, stable fixture,
  API/schema/app-builder docs, and changelog all include the new command. GIF
  or video encoding is not silently fabricated: the report and docs state that
  deterministic PNG frames/contact sheet are the supported core output and an
  external GIF/video encoder is required for those containers.
- `focused`: the three command-unit regressions pass; the 11-frame rendered
  integration passes with four canonical views, four turntable frames, three
  clip samples at 0/0.5/1 seconds, differing adjacent turntable and clip-end
  hashes, all PNG/descriptor files, and an 11-tile contact sheet. The existing
  CAD rendered integration also passes after shared-compositor extraction.
  Durable local proof is
  `target/gate-artifacts/fr05-capture-sequence/capture-sequence-result.json`
  plus its 11 PNGs/descriptors and contact sheet.
- `scoped`: the six-test FR04 CLI output matrix, all 58 stable-contract tests,
  the schema-list CLI golden, the FR05 doctor mutation, remote `cargo fmt --all
  --check`, and live `cargo run -p xtask -- doctor --full` pass. Doctor pins the
  shared CAD/capture utilities, conventions, lifecycle, limits, evidence, help,
  schema, and docs, and rejects a removed explicit prepare step. Local `git
  diff --check` passes.
- `builder/bootstrap`: canonical source is `/home/johannes/projects/scena` on
  branch `main` at `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated remote
  validation path is
  `/home/johannes/.cache/codex-worktrees/scena-scena-fr01-fr04` with target
  `/home/johannes/.cache/codex-targets/scena-scena-fr01-fr04`. Manually copied
  bootstrap hashes match: `AGENTS.md`
  `2a6a3f624549d41f73c246f042eb6bdc6f61d6a2fb5f6911dc3377ddc1b6f3f4` and
  `.codex/skills/**`
  `93257ec7c649725f8ebba630bc638784a8c087bdf3124167d8bffedc744fddd9`.
- `skipped`: no full runtime suite, clippy, rustdoc, browser/WebGPU/native-GPU,
  publish dry-run, or release chain was repeated for this CPU CLI/capture
  slice. No commit, push, tag, merge, release, publish, or other external state
  mutation occurred.

### FR06 — Semantic AOVs (F6)

- [x] Specify node versus primitive/instance IDs, background, transparency,
  strokes, labels, overlays, MSAA resolve, occlusion, depth convention, and
  normal coordinate space.
- [x] Define stable paletted ID image plus legend schema mapped to stable host
  identity without exposing stale handles as persistence.
- [x] Implement deterministic CPU ID/depth/normal outputs first.
- [x] Add overlap/occlusion/transparency/instance truth fixtures.
- [x] Add GPU targets/readback only after lifecycle PF01; prove native,
  WebGPU, and WebGL2 parity.
- [x] Revisit effort estimate after the CPU contract, not before.

FR06 CPU completion ledger (2026-07-17):

- `scope/contract`: `docs/specs/semantic-aov-v1.md` is the accepted CPU-first
  contract. V1 identifies a node plus optional authored instance, not an
  individual primitive/triangle. Palette index zero is transparent background.
  Opaque and alpha-masked prepared triangles use nearest-fragment occlusion;
  alpha-blended/OIT/transmission geometry, strokes, labels, particles, helpers,
  and overlays remain background and are counted. Sampling is one pixel-center
  sample with no MSAA/supersample/post-process resolve. Raw depth is positive
  linear camera distance in scene meters and normals are normalized world-space
  geometric vertex normals.
- `identity/persistence`: `SceneHostCore::capture_semantic_aovs` maps prepared
  `NodeKey` plus optional `InstanceId` to a collision-free 24-bit palette and a
  legend. Host handles and runtime instance IDs are labeled `runtime_scoped`.
  Additive `SceneRecipeBuildV1.instances` / `SceneRecipeBuildInstanceV1` rows
  map authored set/instance IDs to those runtime identities; the CLI enriches
  node, instance, and imported-node legend rows without presenting a stale
  handle as persistent identity.
- `implementation`: prepared triangle state now retains CPU-expanded instance
  identity and semantic material class. `src/render/semantic_aov.rs` consumes
  only current prepared state, applies the active camera, clipping planes,
  section box, culling, alpha cutoff, perspective-correct depth/normal weights,
  deterministic equal-depth tie breaking, and emits raw ID/depth/world-normal
  buffers. Particles and GPU stroke quads are explicitly non-attributed. The
  public capture encodes collision-free RGBA8 IDs, zero-background linear
  near/far gray16 depth, and alpha-valid RGBA8 world normals.
- `public workflow`: `scena recipe aov <recipe.json> --out-dir <dir> [--passes
  id,depth,normal] [--max-imports <n>]` writes `id.png`, `depth.png`,
  `normal.png`, and `scena.semantic_aov_result.v1`. Help success/error families,
  schema catalog/get fixture map, catalog and CLI goldens, stable fixture,
  headless/API/schema/LLM docs, README, and changelog are aligned.
- `focused red`: the SceneHost truth test first failed to compile because
  `capture_semantic_aovs` did not exist. The CLI proof then failed with the real
  `invalid_command` contract because `recipe aov` did not exist.
- `focused green`: `tests/fr06_semantic_aov.rs` passes both tests. The first
  proves an excluded transparent foreground does not steal the center ID, the
  nearer opaque node occludes the rear node, two visible authored instances
  receive distinct palette identities, label/transparent exclusions are
  nonzero, depth/normal hits are finite, and an unchanged repeat capture is
  exactly equal. The second writes all three PNG encodings and proves persistent
  recipe node plus `pair/left` and `pair/right` legend identities. Durable local
  proof is `target/gate-artifacts/fr06-semantic-aov/aov-output/` with the three
  PNGs and `semantic-aov-result.json`.
- `scoped`: one combined remote command passes the six-test FR04 CLI output
  matrix, both FR06 tests, all eight schema CLI tests, and all 58 stable-contract
  tests. Remote `cargo fmt --all --check`, the FR06 doctor mutation, and live
  `cargo run -p xtask -- doctor --full` pass. Doctor pins prepared-state
  ownership, identity/persistence semantics, encoding conventions, CLI/schema/
  docs, and truth tests, and rejects relabeling runtime handles as persistent.
  Local `git diff --check` passes.
- `clippy`: a scoped library/CLI/FR06 clippy attempt reached the whole library
  and stopped on eleven pre-existing/current-goal lint findings outside the new
  FR06 implementation (argument-count/large-enum/needless-borrow findings and
  one pre-existing collapsible block in the touched primitive baker). No FR06
  semantic AOV module, SceneHost API, CLI, manifest mapping, or test lint was
  reported. These broad lint findings remain for the final integration batch;
  clippy is not claimed green here.
- `estimate revisited`: with the CPU contract and fixtures now concrete, the
  remaining GPU item is still `large`, not medium: it requires prepared
  node/instance identity in GPU draw records, dedicated ID/linear-depth/normal
  targets, alpha-mask and exclusion parity, lifecycle-owned attachment and
  readback resources, portable WebGL2 encodings, and the same truth oracle on
  native, WebGPU, and WebGL2. Working estimate is 6-10 focused implementation
  and proof days, dominated by three-backend readback/parity rather than CPU
  rasterization.
- `builder/bootstrap`: canonical source is `/home/johannes/projects/scena` on
  branch `main` at `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated remote
  validation path is
  `/home/johannes/.cache/codex-worktrees/scena-scena-fr01-fr04` with target
  `/home/johannes/.cache/codex-targets/scena-scena-fr01-fr04`. Manually copied
  bootstrap hashes remain `AGENTS.md`
  `2a6a3f624549d41f73c246f042eb6bdc6f61d6a2fb5f6911dc3377ddc1b6f3f4` and
  `.codex/skills/**`
  `93257ec7c649725f8ebba630bc638784a8c087bdf3124167d8bffedc744fddd9`.
- `GPU implementation (2026-07-17)`: opt-in renderer settings now lifecycle-own
  three RGBA8 semantic targets, a depth target, pipelines, and readback state.
  Prepared draw and instance records carry deterministic 24-bit palette IDs;
  the MRT shader emits ID, packed linear depth, and world normal while applying
  alpha-mask, clipping, culling, reversed-Z, and the CPU exclusion contract.
  Dynamic prepare updates rebuild the shared attribution palette. Native and
  browser SceneHost APIs map the raw result through the same runtime-scoped
  legend.
- `GPU readback`: native/headless GPU uses texture-to-buffer mapping; WebGPU
  uses async buffer mapping. WebGL2 cannot portably map those buffers, so it
  blits each GPU target through an sRGB-compensated surface pipeline and uses
  the existing preserved-canvas `readPixels` path. This fixed an observed
  WebGL2 hang and an observed sRGB byte conversion (`ID 1` becoming `13`) rather
  than weakening the parity oracle.
- `GPU focused`: the native headless test
  `fr06_headless_gpu_semantic_aov_matches_cpu_center_truth` passes and caught a
  forward-Z/reversed-Z mismatch before the pipeline fix. The builder browser
  harness captures both backends twice with 2,010 attributed pixels each;
  WebGPU/WebGL2 parity is exact for mask, identity, packed depth, and normals.
  Artifacts are
  `target/gate-artifacts/fr06-semantic-aov/browser/{webgpu-id.png,webgl2-id.png,semantic-aov-browser-proof.json}`.
- `proof boundary`: the browser result above used Chrome SwiftShader on the CPU
  builder. It proves the WebGPU and WebGL2 code paths function and agree, but it
  is not real-GPU evidence. The checklist box remains open until the same
  command and native focused test run on the required real GPU machine.
- `GPU scoped`: all three FR06 integration tests pass after the native GPU test
  was made fail-closed on adapter construction. The FR06 doctor mutation and
  live `doctor --full` pass; doctor pins option/lifecycle ownership, GPU draw
  and instance attribution, MRT shaders, native/WebGPU/WebGL2 readback,
  bindings, and parity evidence. The native prepare module was split before
  its architecture size budget and lifecycle pins now follow the new
  `headless_target` owner. `cargo fmt --all --check` and local
  `git diff --check` pass.
- `hardware-lane readiness (2026-07-18)`: native strict mode now parses the
  renderer's capability report and accepts only discrete, integrated, or
  virtual hardware GPUs while rejecting known software identities. The browser
  harness records the complete capability report and applies the shared strict
  adapter evaluator to both WebGPU and WebGL2. CI and release workflows now run
  FR06 on both browser backends diagnostically/strictly according to their
  existing lane policy; the manual hardware workflow runs native plus both
  browser backends with strict hardware flags. The focused native FR06 GPU
  parity test passes 1/1 in 6.88 seconds on the builder as behavioral evidence,
  and the FR06 workflow mutation test plus full doctor pass. The GitHub
  repository currently reports zero registered Actions runners, so no new
  hardware artifact exists and the FR06 checkbox remains open.
- `physical WebGL2 execution (2026-07-18)`: the owner-authorized strict
  WebGL2-only browser lane passed on Broadcom V3D 7.1.10.2/Chromium 148 in
  11.3 seconds with 2,011 attributed hits, 2,011 finite hit depths, and an
  exact deterministic repeat. The adapter evaluator passed with
  `IntegratedGpu`. The artifact
  `target/gate-artifacts/fr06-semantic-aov/browser/semantic-aov-browser-proof.json`
  has SHA-256
  `4944772d9ab7f9a981f0daad974350d970e15d109d5a66ac9018fd23a8de7a14`
  and correctly records `complete_backend_set=false`, `parity=null`, and
  `release_evidence=false`. Chromium exposes no WebGPU adapter on this host;
  Firefox obtains a WebGPU adapter/device and completes SceneHost construction,
  asset import, and prepare, but the first async semantic readback does not
  complete. Therefore physical WebGPU/native parity is still unproven and the
  FR06 checkbox remains open.
- `skipped`: no full runtime suite, rustdoc, publish dry-run, or release chain
  was repeated for this focused GPU slice. No commit, push, tag, merge, release,
  publish, or other external state mutation occurred.
- `complete real-GPU proof (2026-07-19)`: the combined Windows lane proves
  WebGPU and WebGL2 on the physical Intel Arc Pro adapter and native Vulkan on
  the same integrated GPU. Browser parity has 2,011 common hits, mask
  agreement `1`, identity agreement `1`, maximum depth error `0`, and minimum
  normal dot `1`; both ID PNGs are byte-identical at SHA-256
  `d86c52b7543ba9070c5f64d71b73e842c6bcf5886bd397821ff58deb01652a8e`.
  Native CPU/GPU center identity is `1/1`; depth is
  `0.9750000834465027/0.974989652633667` meters against a `0.001`-meter
  tolerance, and the GPU normal is within the `0.01` component tolerance.
  Browser and native proof JSON SHA-256 values are respectively
  `1cc89890f67cc878988143264bcff603c26ae73e70d85c8590598e9a054cbfa6`
  and `ab6593f35921bfb822fdf6fd580e7e9af6220597afa4a54f4052858ccaeedbd2`;
  both record required hardware and release evidence. This supersedes the
  earlier SwiftShader and partial V3D proof boundaries without relabeling
  those historical diagnostic artifacts.

### FR07 — Structural and attributed diff (F7)

- [x] Ship typed scene/recipe semantic diff independently of AOV work; keep
  generic JSON diff outside renderer ownership.
- [x] Define node/material/camera/add/remove/reorder and tolerance semantics.
- [x] Reuse existing aggregate capture diff for non-attributed visual changes.
- [x] Add node attribution only after FR06 ID AOV semantics are accepted.
- [x] Report unattributed/ambiguous transparent/anti-aliased regions honestly.
- [x] Remove universal uniqueness marketing unless a dated matrix supports it.

FR07 completion ledger (2026-07-17):

- `scope/ownership`: structural semantics live in `scene::recipe::diff`; the
  public `scena diff` workflow and rendered/AOV composition live under
  `bin/scena/diff`. Generic arbitrary-JSON diffing remains outside scena
  renderer ownership. The canonical RFC already includes authoring helpers,
  diagnostics, deterministic proof artifacts, and semantic AOVs, so this
  recipe-specific workflow does not add simulation/domain scope.
- `typed contract`: `scena.scene_recipe_diff.v1` reports stable material,
  node, camera, and recipe-level identity; add/remove/modify/reorder kinds;
  deterministic field paths and before/after order; and an explicit finite
  non-negative numeric tolerance. `scena diff <before> <after>` stays
  renderer-free and reports zero renderer/prepare/render/capture calls.
- `rendered contract`: `--render --out-dir <dir>` builds and renders both
  recipes, reuses `compare_captures_with_tolerance` and
  `scena.capture_baseline.v1`, captures both semantic ID AOVs, and writes
  `before.png`, `after.png`, `diff.png`, and
  `recipe-diff-result.json`. Persistent recipe node, imported-node, and
  authored-instance candidates come from each build manifest rather than
  treating runtime handles as persistence.
- `honest attribution`: every changed color pixel is exactly partitioned as
  attributed, ambiguous, or unattributed. Semantic identity boundary pixels
  are ambiguous. Background/zero-ID pixels are unattributed. Different
  identities are ambiguous. If either AOV reports any transparent,
  transmission/unattributed, overlay, stroke, label, or GPU-instance
  exclusion without a per-pixel mask, all otherwise attributed candidates are
  downgraded to ambiguous; the proof caught the prior false assignment of a
  transparent material color change to the opaque node behind it.
- `public surface`: CLI machine help declares success and validation/build/
  argument-error schemas. `scena.scene_recipe_diff_result.v1` is cataloged
  with a stable fixture and CLI golden. Schema/API/LLM workflow docs, README,
  and changelog describe the command and its limitations. Documentation makes
  no universal competitive uniqueness claim without a dated, source-backed
  product matrix.
- `focused red`: the first remote FR07 run kept the typed-core test green but
  failed both new CLI tests with structured `invalid_command` because
  `scena diff` did not exist. The transparency proof then failed with 225
  pixels falsely attributed to the opaque `back` node while both AOVs reported
  12 excluded transparent triangles.
- `focused green`: all five `tests/fr07_recipe_diff.rs` tests pass. They cover
  typed identity/fields/tolerance/order, renderer-free CLI execution, real
  validation/build failure families, rendered aggregate diff plus persistent
  attribution, and the excluded-transparency regression. The representative
  rendered artifact has 527 changed pixels partitioned into 390 attributed,
  137 ambiguous edge pixels, and zero unattributed pixels; the transparency
  artifact has zero attributed pixels.
- `doctor`: `FR07-RECIPE-DIFF` pins typed ownership, CLI aggregate/AOV reuse,
  persistent mapping, ambiguity/unattributed reasons, schema/docs/fixtures,
  and all focused tests. Its mutation test changes anti-aliased edges from
  ambiguous to attributed and is rejected. `doctor --full` initially caught
  the schema catalog at 501 significant lines; extracting the catalog-row
  type to `schema_catalog/entries.rs` restored the 500-line architecture
  budget, after which full doctor passed.
- `scoped`: remote `cargo fmt --all --check`; 6 FR04 command-schema tests; 8
  schema CLI tests; 58 stable-contract tests; all 5 FR07 tests; the focused
  xtask mutation test; and live `cargo run -p xtask -- doctor --full` pass.
  Local `git diff --check` passes.
- `builder/bootstrap`: canonical source is `/home/johannes/projects/scena` on
  branch `main` at `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated
  remote validation is
  `/home/johannes/.cache/codex-worktrees/scena-scena-fr07` with target
  `/home/johannes/.cache/codex-targets/scena-scena-fr07`. Manually copied
  bootstrap hashes match: `AGENTS.md`
  `2a6a3f624549d41f73c246f042eb6bdc6f61d6a2fb5f6911dc3377ddc1b6f3f4`
  and `.codex/skills/**`
  `93257ec7c649725f8ebba630bc638784a8c087bdf3124167d8bffedc744fddd9`.
- `full/skipped`: no full cargo test/clippy/rustdoc/browser GPU/publish/release
  chain was repeated because this CPU recipe/CLI slice has focused rendered
  proof and its affected CLI/schema/doctor gates are green. FR06 real-GPU
  parity remains a separate open box. No commit, push, tag, merge, release,
  publish, or other external-state mutation occurred.

### FR08 — Recipe anchors/connectors/bounds/named states (F8)

- [x] Specify persistent recipe IDs and build-manifest mapping for every owner.
- [x] Define local/source/world unit semantics consistently with C05.
- [x] Define connector compatibility/snap failure diagnostics and bounds source
  (authored, imported, computed).
- [x] Define named-state snapshot contents, inheritance, animation interaction,
  and missing-target policy.
- [x] Add validation, authoring, execution, manifest, round-trip, and rendered
  placement/state proof before removing `unsupported_feature`.

FR08 completion ledger (2026-07-18):

- `scope/spec`: `docs/specs/recipe-spatial-state-v1.md` accepts the four
  renderer-owned recipe sections while keeping sequencing, simulation,
  robotics, PLC/domain behavior, and hidden runtime persistence outside scena.
  Every feature row has a caller-owned persistent ID; numeric node/import/
  connector handles remain build-scoped outputs only.
- `typed targets/units`: closed `node`, `import_root`, and `import_node` targets
  resolve through exact recipe/import identity. Authored positions, bounds,
  snap tolerance, clearance, and connection offsets use local/world scene
  meters after the C05 import conversion boundary. Imported aliases preserve
  source-unit and source-coordinate metadata without applying a second
  conversion.
- `anchors/connectors/bounds`: validation and execution now own authored and
  imported anchor/connector aliases, metadata-compatible `connect_by_key`
  mating, structured compatibility/snap/roll/alignment failures, and authored,
  imported, or computed bounds. Authored bounds can only attach to empty group
  nodes and cannot replace geometry- or asset-owned bounds.
- `named states`: typed transform/tint/visibility snapshots support acyclic
  single inheritance with deterministic child override, exactly zero or one
  active state, fail-closed missing targets, and explicit rejection of
  transform channels targeting recipe-animated nodes. Build stores every
  state through SceneHost and applies the active state once after mating.
- `manifest/schema`: `scena.scene_recipe_build.v1` reports persistent anchor,
  connector, connection, bounds, and named-state rows. The recipe field model,
  schema/API/LLM docs, README, changelog, CLI catalog/golden, and stable fixtures
  expose the accepted sections. The previously uncataloged FR07 structural
  `scena.scene_recipe_diff.v1` schema was also registered when full doctor
  caught the stale evidence catalog.
- `focused red`: the first remote FR08 test failed because all four sections
  still emitted `unsupported_feature` and group nodes without geometry/material
  were rejected. This pinned both the missing typed execution surface and the
  required non-renderable bounds owner before production implementation.
- `focused green`: all five `tests/fr08_recipe_spatial_state.rs` tests pass.
  They cover every target/source variant and JSON round trip; exact imported
  alias metadata; persistent manifest mapping; connector compatibility,
  snap/roll, bounds, inheritance, missing target, animation conflict, and
  authored-bounds override failures; plus actual mating and active-state
  application.
- `rendered proof`: remote artifacts under
  `target/gate-artifacts/fr08-recipe-spatial-state/` contain `control.png`,
  `feature.png`, and `proof.json`. The proof schema is
  `scena.fr08_recipe_spatial_state_proof.v1`, its build manifest is `ok:true`,
  the connection is `applied`, and the active named state plus mate change 364
  of 12,288 headless CPU pixels. The zero-tolerance comparison intentionally
  reports `failed` because this proof asserts the declared state changes the
  control render rather than claiming equality.
- `doctor/architecture`: `FR08-RECIPE-SPATIAL-STATE` pins typed owners,
  fail-closed diagnostics, documentation, schema fields, and all five focused
  tests; its mutation restores `unsupported_feature` and is rejected. Full
  doctor also forced spatial validation/host execution into owner-specific
  modules, moved LOD and build-policy helpers out of over-budget modules,
  removed the catch-all helper name, and leaves every affected source owner at
  or below the 500-significant-line limit. Live `doctor --full` passes.
- `scoped`: remote 5/5 FR08 tests; the focused xtask mutation test; 8/8 schema
  CLI tests; 59/59 stable-contract tests; the exact repaired build-manifest
  golden test; `cargo fmt --all --check`; and live `cargo run -p xtask --
  doctor --full` pass. The affected 61-test recipe-contract bin had 60 passes
  and one stale `instances` golden failure; the fixture was corrected and its
  exact test passed. Local `git diff --check` passes.
- `builder/bootstrap`: canonical source is `/home/johannes/projects/scena` on
  branch `main` at `bea2a36f5a5e5f5610fa578f1915f137e432281c`; isolated
  remote validation is
  `/home/johannes/.cache/codex-worktrees/scena-scena-fr07` with target
  `/home/johannes/.cache/codex-targets/scena-scena-fr07`. Manually copied
  bootstrap hashes match: `AGENTS.md`
  `2a6a3f624549d41f73c246f042eb6bdc6f61d6a2fb5f6911dc3377ddc1b6f3f4`
  and `.codex/skills/**`
  `93257ec7c649725f8ebba630bc638784a8c087bdf3124167d8bffedc744fddd9`.
- `full/skipped`: no full workspace clippy/test/rustdoc/browser GPU/publish or
  release chain was repeated for this CPU recipe slice. Focused rendered,
  schema, formatting, mutation, and full-doctor evidence cover the changed
  surfaces; the one-time integrated release chain remains the final checkpoint.
  No commit, push, tag, merge, release, publish, or other external-state
  mutation occurred.

### FR09-FR13 owner decision request (2026-07-18)

The canonical RFC is owner-ratified and makes renderer quality, glTF import,
assets, capture, and domain-neutral viewer workflows in scope, but it does not
specifically ratify these large optional feature proposals or assign them to a
v1.7.2 remediation milestone. On 2026-07-18 the project owner approved all
recommended dispositions below. The implementation boxes are therefore
checked as closed by explicit owner-approved deferral, not as implemented
features. These dispositions preserve structured unsupported/unavailable
behavior and forbid marketing claims while deferred.

| Scope | Recommended owner disposition | Revisit trigger |
|---|---|---|
| FR09 Draco/remaining glTF modes | Defer. Keep decode in `assets`, retain explicit unsupported primitive-mode errors, and do not rank Draco without evidence. | At least three distinct real failed assets or customer issues, followed by a dated decoder/license/WASM/security evaluation. |
| FR10 spot/point shadow maps | Defer as two separate renderer projects; projected spot shadows must precede cubemap point shadows. | An accepted product use case naming required light types/backends and a resource/proof budget. |
| FR11 GPU weighted-blended OIT | Defer; keep the GPU capability unavailable rather than implying CPU OIT parity. | An accepted overlapping-transparency use case plus an RFC addendum covering post/MSAA/transmission/resource interactions. |
| FR12 glTF/GLB export | Keep export out of core `scena` for now and evaluate an optional companion crate/tool first. | A concrete round-trip/export consumer and an owner-approved supported-subset contract with explicit loss reporting. |
| FR13a section capping | Defer as a CAD feature with closed-solid/nonmanifold limitations. | An accepted CAD cutaway use case and cross-backend proof budget. |
| FR13b KTX2 cubemap environments | Defer independently of 2D KTX2 material textures. | Real six-face/HDR assets plus an accepted face/mip/orientation/cache contract. |
| FR13c SDF/MSDF text | Defer; retain the corrected finding that no public SDF/MSDF API exists. | An accepted label-quality/zoom requirement and atlas ownership contract. |
| FR13d higher-precision capture | Defer 16-bit/float/EXR output. | A concrete scientific/CAD interchange consumer with numeric and external-reader acceptance criteria. |
| FR13e watch mode | Defer even though transactional C02 is complete; do not add hidden rebuild/fetch work. | An accepted CLI authoring workflow with debounce, policy, last-good, and error-report semantics. |
| FR13f international text | Defer as a complete text-system scope, not a shaping-only patch. | Named scripts/fonts plus accepted fallback, bidi, breaking, caret, atlas, and native/browser proof requirements. |

Owner decision: all dated deferrals were approved in the remediation goal
thread on 2026-07-18. Reopening any row requires its stated trigger and a new
scope decision; no deferred capability may be claimed as implemented or used
as release evidence.

### FR09 — Draco and remaining glTF import demand (F9)

- [x] Collect real failed-asset telemetry/issues before ranking Draco or other
  codecs as the top blocker.
- [x] Evaluate maintained decoder options for license, pure Rust/WASM support,
  malformed-input behavior, size/build cost, determinism, and upstream health.
- [x] Keep feature-gated decode in `assets`; renderer must not own/fetch codecs.
- [x] Add official/known-bad fixtures and native/WASM rendered proof.
- [x] Treat triangle strip/fan/line/point modes as separate primitive/renderer
  contracts with explicit unsupported diagnostics until implemented.
- [x] Add a tailored spec-gloss fallback diagnostic.
- [x] Complete native WebP C03 before `EXT_texture_webp` rebinding.

### FR10 — Spot and point shadow maps (F10)

- [x] Split projected spot shadows from cubemap point shadows.
- [x] Define authoring knobs/defaults, map allocation, filtering/bias, receiver
  sampling, capability status, diagnostics, stats, and destruction.
- [x] Keep prepare/render lifecycle explicit and add source-derived doctor
  ownership checks.
- [x] Add on/off, acne/peter-panning, range/cone, six-face seam, context-loss,
  native/WebGPU/WebGL2 rendered proof.

### FR11 — GPU weighted-blended OIT (F11)

- [x] Define accumulation/revealage attachments and capability requirements.
- [x] Integrate transparent sorting, transmission, post, MSAA, resize, and
  context/device loss.
- [x] Account all resources/stats under C09/PF01.
- [x] Prove order invariance for overlapping surfaces on native, WebGPU, and
  WebGL2 or fail closed per backend.

### FR12 — glTF/GLB export scope decision (F12)

- [x] Obtain RFC owner ratification that export belongs in core scena or choose
  an optional companion crate/tool.
- [x] Define supported scene/material/animation/skin/morph/texture/extension
  mapping and explicit unsupported-feature report; no silent drops.
- [x] Specify buffer/image embedding, URI/policy behavior, units, coordinates,
  stable naming, and deterministic output.
- [x] Add export-reimport semantic/visual round-trip fixtures and official
  validator proof.
- [x] Re-estimate as L/XL after scope; do not assume import architecture makes
  reverse serialization small.

### FR13 — Separate parity candidates (F13)

#### FR13a — Section-box capping

- [x] Define closed-solid/cap material, winding, multiple planes, concave
  intersections, transparent objects, and nonmanifold limitations.
- [x] Add CPU/native/WebGPU/WebGL2 CAD cutaway proof.

#### FR13b — KTX2 cubemap environments

- [x] Treat six-face KTX2 decode/transcode/layout as new work; current material
  path rejects face count greater than one.
- [x] Define mip/face orientation, DFD/HDR limitations, sidecar/cache identity,
  native/WASM support, and rendered IBL parity.

#### FR13c — SDF/MSDF text

- [x] Correct the review: no public `LabelDesc::sdf()`/`msdf()` API exists.
- [x] Define public raster mode, atlas generation/cache, zoom thresholds,
  derivatives, outline/shadow behavior, fallback, stats, and proof before adding
  API.

#### FR13d — Higher-precision capture

- [x] Define linear-versus-display encoding, 16-bit integer/float and EXR
  metadata, tone-map bypass, alpha, ICC interaction, and CLI schema.
- [x] Add numeric round-trip and external-reader compatibility proof.

#### FR13e — Watch mode

- [x] Complete C02 transactional hot reload first.
- [x] Reuse the native watcher, debounce, policy, manifest, renderer prepare,
  and output ownership; do not create hidden fetch/upload inside render.
- [x] Report each rebuild success/failure and preserve last-good output on
  failure.

#### FR13f — International text

- [x] Treat Unicode coverage/font fallback, bidi, shaping, line breaking,
  clusters/caret mapping, and atlas lifecycle as one explicit text-system
  scope; “complex shaping” alone does not solve CJK.
- [x] Preserve fail-closed behavior until each declared script/font surface has
  native/browser rendered proof.

## 9. Remote-builder and operational remediation

### O01 — Reconcile documented and actual builder checkout

- [x] Decide whether `$HOME/projects/scena` should be provisioned as the shared
  canonical remote checkout or update repository instructions/remote-builder
  skill to the actual maintained path.
- [x] Add a preflight that reports a missing shared checkout before attempting
  cargo, then chooses an isolated snapshot without ambiguity.
- [x] Keep isolated task snapshots and target caches task-scoped; never delete
  unrelated caches to recover disk space.
- [x] Preserve the manual `AGENTS.md`/`.codex/skills/**` copy-and-hash gate for
  every remote destination.

O01 remediation ledger (2026-07-17):

- `decision`: the maintained builder does not provision
  `$HOME/projects/scena`. Repository instructions and the remote-builder skill
  now treat it only as an observed legacy/shared path and default every task to
  `$HOME/.cache/codex-worktrees/scena-<task-slug>` paired with
  `$HOME/.cache/codex-targets/scena-<task-slug>`.
- `focused red`: `cargo test -p xtask tests_33 -- --nocapture` first failed at
  compile time because `check_review_provenance_contracts` and
  `check_remote_builder_bootstrap_contracts` did not exist. After
  implementation, the known-bad O01 fixture rejects instructions that assume
  the missing shared checkout and omit isolated fallback/manual agent-file
  bootstrap; both D04/O01 mutations pass 2/2.
- `live preflight`: running
  `ssh scena-builder 'bash -s -- d04-o01' <
  scripts/scena_remote_builder_preflight.sh` reports
  `shared_checkout_status=missing`, `validation_mode=isolated`,
  `validation_path=/home/johannes/.cache/codex-worktrees/scena-d04-o01`, and
  `cargo_target_dir=/home/johannes/.cache/codex-targets/scena-d04-o01` before
  sync or cargo. The script performs disk reporting and has no deletion path.
- `bootstrap`: the fresh isolated destination received an explicit manual copy
  of root `AGENTS.md` and then the complete `.codex/skills/**` tree after the
  normal source sync. Remote hashes match canonical:
  `AGENTS.md=2a6a3f624549d41f73c246f042eb6bdc6f61d6a2fb5f6911dc3377ddc1b6f3f4`
  and complete skills
  `93257ec7c649725f8ebba630bc638784a8c087bdf3124167d8bffedc744fddd9`.
- `scoped`: the checked-in preflight passes `bash -n` and executable-bit
  inspection locally and remotely; `cargo fmt --all --check` passes remotely;
  `cargo run -p xtask -- doctor --full` reports `mode=Full status=pass` with
  the new D04 review-provenance and O01 bootstrap rules active.
- `skipped`: this is operational policy/tooling, not renderer behavior. No
  browser, GPU, renderer, clippy, rustdoc, publish, or release lane was rerun.
  No shared checkout was provisioned and no unrelated remote cache or checkout
  was deleted.

Audit-time remote evidence:

- [x] Shared documented path was absent, so validation used isolated snapshot
  `/home/johannes/.cache/codex-worktrees/scena-full-review-audit`.
- [x] Task target cache was
  `/home/johannes/.cache/codex-targets/scena-full-review-audit`.
- [x] Local source was synced excluding `.git`, `target`, `AGENTS.md`, and
  `.codex`; agent files were then manually copied and hash-verified before any
  cargo command.
- [x] Source snapshot was local `main@bea2a36`; the isolated destination had no
  git metadata by design.

## 10. Audit validation ledger

### Focused

- [x] Remote B1 reproducer:
  `scena validate-recipe` received
  `{"schema":"scena.scene_recipe.v1","colors":{"c":"€abc"}}` through
  `/dev/stdin`; exit was 101 with a non-char-boundary panic at
  `src/material/color.rs:214`.
- [x] Remote `cargo run -p xtask -- doctor --full` on the fresh isolated
  snapshot failed with eight findings:
  - missing/unreadable `target/gate-artifacts/m5-benchmarks.json` twice;
  - missing/unreadable `target/gate-artifacts/m5-public-api-freeze.json` twice;
  - four `ARCH-KISS-SIZE` findings for the files in S10.
- [x] Remote `doctor --docs` passed despite sixteen currently absent pinned
  document paths, proving the S6 fail-open family.
- [x] Remote `doctor --architecture` also included the unrelated release
  artifact failures, proving submode scope leakage.
- [x] The first remote documentation check of this checklist rejected its
  literal references to the uncatalogued CLI help/version contracts. The
  wording was corrected so an open finding does not falsely declare those
  schemas registered; the same `doctor --docs` command then passed.

### Scoped static proof

- [x] Every B1-B22, S1-S10, P1-P12, and F1-F13 claim was traced to source,
  tests, workflows, docs, or official glTF extension/spec text.
- [x] Stable schema catalog count, workflow commands, xtask test coverage,
  missing docs, ignored artifacts, tag/HEAD relationship, public APIs, and
  newly found hot paths were mechanically inspected.
- [x] No local cargo/build/test/browser command was run.

### Full

- [x] The integrated remediation checkpoint ran on the bootstrapped isolated
  remote snapshot. It includes strict formatting/clippy, all native unit and
  integration targets, 325 xtask tests, full doctor, warning-denied rustdoc,
  examples, WASM all-features compile, the provenance-bearing M5 public-API
  producer, and locked publish dry-run. Required real-hardware GPU/browser
  artifacts, release staging, and RFC-owner dispositions remain explicitly
  open below rather than being inferred from builder or workflow-source proof.

### Skipped and why

- [x] Broad cargo/clippy/test/doc/publish gates were skipped because they do not
  validate a read-only review ledger and the fresh doctor baseline is already
  known red.
- [x] GPU/browser visual reruns were skipped because no visual implementation
  changed; existing workflow/source evidence was audited instead.
- [x] Timing claims remain explicitly unmeasured where no existing benchmark
  proves them.

## 11. Integration checkpoint and final definition of done

### 11.1 Dependency order

1. D01-D03: make xtask/release/doctor evidence trustworthy and hermetic.
2. C01-C05: untrusted input, transactional import, textures, glTF semantics,
   and units.
3. C06-C13: transforms, handles, timeline, lifecycle diagnostics, overlays,
   docs, picking correctness, and strict GPU construction.
4. Q01-Q07: close live visual/browser/GPU/quality proof debt.
5. PF00: repair and populate representative performance baselines.
6. PF01-PF10: lifecycle first, then clones/assets/transforms, shared spatial
   acceleration, tangent/bake/parallel work, and measured tail waste.
7. FR01-FR05: agent-surface work after contract truth is stable.
8. FR06-FR13: differentiated/parity work only after RFC/demand decisions and
   stated dependencies.

### 11.2 Full checkpoint gates

Run once the integrated correctness/proof batch is ready, on a bootstrapped
remote snapshot and appropriate real GPU/browser machines:

- [x] `cargo fmt --all --check`.
- [x] Focused tests named by every completed item, including expected-red
  before/green after evidence.
- [x] `cargo test -p xtask`.
- [x] `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] `cargo test` plus the feature-specific integration targets touched.
- [x] `cargo run -p xtask -- doctor --full` from a fresh target/artifact state.
- [x] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` for public
  API/docs changes.
- [x] Required WebGL2/WebGPU/native GPU rendered-output and parity lanes for
  touched visual/backend behavior.
- [x] Performance distributions/baseline comparison for any PF item.
- [x] `cargo publish --dry-run` and semver/public-API checks only at a
  release-ready/public-API checkpoint.
- [x] Stage release artifacts from source-provenance-bearing lane outputs and
  prove all known-bad substitutions are rejected.

Validation ledger (2026-07-18 integrated checkpoint):

- `focused`: the final PF08/Q01 correction uses the greater of projected edge
  pixels and decoded texture edge texels, capped at 48 subdivisions. The
  original committed WaterBottle golden remained unchanged and passed at
  SSIM `0.999664306640625`, RMSE `0.200922`, and maximum channel error `14`;
  every known-bad mutation was rejected. PF08 focused parity, the M9 cap
  contract, the corrected Round E source pins, and both allocation assertions
  passed. The M9 allocation harness now counts on the calling thread, removing
  process-global cross-test pollution without weakening its zero-allocation
  contract.
- `scoped`: on the bootstrapped isolated remote copy
  `/home/johannes/.cache/codex-worktrees/scena-hardware-proof`, with external
  target `/home/johannes/.cache/codex-targets/scena-hardware-proof`,
  `cargo test -p xtask` passed 325/325, the complete M9 target passed 28/28,
  `cargo check --target wasm32-unknown-unknown --all-features`,
  `cargo check --examples`, the provenance-bearing M5 API producer, and all
  touched feature targets passed.
- `full`: `cargo fmt --all --check`, warning-denied all-target Clippy, the full
  `cargo test` graph, 62 passing doctests plus four passing compile-fail
  doctests, warning-denied all-features rustdoc, and locked publish dry-run all
  passed. Crates.io correctly reported that version 1.7.2 already exists while
  dry-run prevented an upload.
- `doctor`: canonical `/home/johannes/projects/scena` and remote destination
  `/home/johannes/.cache/codex-worktrees/scena-doctor-fresh` use branch `main`
  and source HEAD `bea2a36f5a5e5f5610fa578f1915f137e432281c`. `AGENTS.md`
  (`2a6a3f62...`) and the complete `.codex/skills` tree (`93257ec7...`) were
  manually copied and hash-verified. The destination contained no `target`
  directory; its source-only `xtask doctor --full` completed with
  `mode=Full status=pass` using only an external task target cache.
- `hardware/local`: with explicit owner authorization, the physical V3D host
  produced strict PF01 and FR06 WebGL2 artifacts. Both are intentionally
  partial and record `complete_backend_set=false` plus
  `release_evidence=false`. Native V3D prepare stalls in Mesa pipeline
  compilation; Chromium exposes no WebGPU adapter; Firefox WebGPU obtains a
  device but renderer readback does not complete. These focused failures keep
  the native/WebGPU/WebGL2 gates open.
- `hardware/windows`: Edge on the Windows laptop completed the one-shot strict
  V7 run on the physical Intel Arc Pro adapter. PF01 passed all five phases on
  WebGPU, WebGL2, and native Vulkan; PF02 native attached PresentOnly passed
  with every readback/blocking/allocation counter at zero; FR06 passed exact
  WebGPU/WebGL2 parity and strict native CPU/GPU parity. The combined summary
  records `status=passed`, `hardware_evidence=true`, both browser backends,
  native surface, native semantic AOV, hardware-attestation success, and
  hashes for every required artifact. It records `release_evidence=false`
  because no exact source commit binds the collected files. The received
  archive and durable local copies were independently hash-verified before
  this hardware gate was closed.
- `performance`: all ten PF00 workloads passed as optimized 100-sample
  distributions in one 5294.06-second run; the immutable raw files and
  corrected nonrelease aggregate are hash-bound above. PF10 adds one same-run
  interleaved 100-pair comparison and records both the dense benefit and sparse
  forced-enable cost. Performance measurement and physical-GPU behavior are
  complete; exact-commit release provenance remains a separate staging
  requirement.
- `owner dispositions`: the project owner explicitly approved every dated
  FR09-FR13 recommended deferral on 2026-07-18. All 33 implementation-shaped
  boxes are closed by disposition, not by implementation, and the deferred
  capabilities remain forbidden from support or release claims until their
  stated reopen triggers are met.
- `release artifact staging (2026-07-19)`: the focused duplicate-producer
  regression
  `q01_stage_source_prefers_the_finalized_headless_cpu_producer` first selected
  the unfinished macOS Q01 copy and then passed after source ranking was bound
  to the Linux/headless producer. Replaying the complete artifact download from
  GitHub run `29697027666` with the corrected staging executable consumed the
  finalized Q01 result and reached only the stable external-review boundary,
  `RELEASE-REVIEWS-MISSING`. The bootstrapped remote snapshot
  `/home/johannes/.cache/codex-worktrees/scena-scena-release-guardrails`, with
  target `/home/johannes/.cache/codex-targets/scena-scena-release-guardrails`,
  passed that focused regression, all 337 `xtask` tests, warning-denied scoped
  Clippy, formatting, and `doctor --full`. The full D01 negative matrix remains
  green, including wrong-backend, wrong-lane, missing-result, stale-hash,
  substituted-output, synthetic-provenance, missing-review, automation-review,
  open-finding, commit-mismatch, tampered-report, and missing-signoff fixtures.
- `GitHub`: CI run `29699581847` at exact commit
  `bf0ba170010ec0d913cdeefcaf753d919bec3562` passed Linux native/headless,
  Linux browser WebGL2, Linux browser WebGPU, Windows DX12, macOS Metal, WASM
  package, and headless 4K performance. Its dependent `Pre-merge release
  evidence integrity` job downloaded those source-provenance-bearing lane
  outputs, staged them successfully through every source-evidence check, and
  passed only when the resulting report ended at `RELEASE-REVIEWS-MISSING`.
  This closes artifact staging without pretending that branch CI can approve
  its own commit.
- `final evidence separation (2026-07-19)`: canonical local `main` was clean
  and matched `origin/main` at
  `bf0ba170010ec0d913cdeefcaf753d919bec3562` when the deciding CI run passed.
  The separately supplied Windows laptop result remains physical-hardware
  proof, while run `29699581847` is GitHub workflow proof. The public GitHub
  Latest release and crates.io version remain `v1.7.2`/`1.7.2`; no `v1.8.0`
  tag or release existed at checklist closeout, so publication is not inferred
  from the completed remediation or CI evidence.
- `skipped`: no gate was rerun for this evidence-only checklist edit because
  it changes no code, workflow, schema, or artifact-selection surface after
  run `29699581847`. Final `v1.8.0` release readiness and publication remain a
  separate operation requiring the independently authored review bundle
  defined by `docs/specs/release-reviews.md`; no review identity or approval
  was synthesized to close this checklist.

### 11.3 Completion conditions

- [x] Every original claim ID has a final verdict and a closed disposition;
  refuted B11/B15 remain spec/behavior regression locks, not “fixed” behavior.
- [x] Every N01-N21 finding has an owner, focused proof, implementation or
  explicit accepted deferral, and validation ledger.
- [x] No public input path panics, silently corrupts state, or reports a
  backend/resource/destruction action that did not happen.
- [x] Import/hot reload is atomic and bounded; non-meter world semantics and
  deformed picking are correct.
- [x] Release evidence cannot be synthesized, relabeled, or accepted from
  generic nonblack/fake image data.
- [x] Doctor fails closed on missing canonical inputs and its mutation tests run
  in CI.
- [x] Visual claims bind live renderer output to feature-specific metrics and
  source provenance.
- [x] Performance claims are supported by representative distributions and
  honest thresholds, not arithmetic estimates or fabricated zeroes.
- [x] Optional features are implemented only after RFC/demand approval and all
  owner/proof/lifecycle requirements.
- [x] Final handoff distinguishes local, remote-builder, GitHub workflow, and
  published-release evidence. No layer is inferred from another.

## 12. Claim-to-work crosswalk

Use this as the completeness guard when items are split into issues/PRs.

- B1 -> C01; B2 -> C02; B3 -> C03; B4-B6 -> C04; B7-B8 -> C06;
  B9 -> C08; B10 -> C09/PF01; B11 -> C05 regression lock; B12 -> C07;
  B13 -> C04; B14 -> C06; B15 -> C03 regression lock; B16 -> C01;
  B17 -> C10; B18 -> C01; B19 -> C09; B20 -> C04; B21-B22 -> C11.
- S1 -> Q01/D01; S2 -> Q02; S3 -> Q03; S4 -> Q04/D01;
  S5 -> Q05; S6 -> D02; S7-S8 -> Q06/D01; S9 -> Q07/D02;
  S10 -> D03.
- P1 -> PF06; P2 -> PF03; P3 -> PF04; P4 -> PF05; P5 -> PF07;
  P6 -> C12/PF06; P7 -> PF08; P8 -> PF09; P9 -> PF02;
  P10 -> PF01; P11 -> PF10; P12 -> PF00.
- F1 -> FR01; F2 -> FR02; F3 -> FR03; F4 -> FR04; F5 -> FR05;
  F6 -> FR06; F7 -> FR07; F8 -> FR08; F9 -> FR09; F10 -> FR10;
  F11 -> FR11; F12 -> FR12; F13 -> FR13a-FR13f.
- N01 -> C01; N02 -> C02; N03 -> C05; N04 -> C03; N05 -> C12;
  N06 -> PF10; N07-N11 -> D01; N12 -> D02; N13 -> Q07;
  N14 -> C13/Q06; N15 -> C11; N16 -> D04; N17 -> FR01/FR04;
  N18 -> O01; N19 -> C11/Q01/D02; N20 -> D03/PF00; N21 -> D01.
