const assert = require("assert");
const crypto = require("crypto");
const fs = require("fs");
const os = require("os");
const path = require("path");

const {
  validateProofRoot,
} = require("./windows_complete_hardware_proof_validation.js");

function resources(seed) {
  return [seed, seed + 1, seed + 2, seed + 3, seed + 4, seed + 5];
}

function browserGpu() {
  return {
    source: "chromium-cdp-system-info",
    devices: [{
      vendor_id: 32902,
      device_id: 32085,
      vendor_string: "",
      device_string: "Intel(R) Arc(TM) Pro Graphics",
      driver_vendor: "Intel",
      driver_version: "32.0.101.8517",
    }],
    aux_attributes: {
      gl_vendor: "Google Inc. (Intel)",
      gl_renderer: "ANGLE (Intel, Intel(R) Arc(TM) Pro Graphics, D3D11)",
    },
    feature_status: { webgl: "enabled", webgpu: "enabled" },
  };
}

function capabilityReport(backend, deviceType = "IntegratedGpu") {
  return {
    schema: "scena.capability_report.v1",
    capabilities: {
      backend,
      gpu_device: true,
      surface_attached: true,
    },
    adapter: {
      name: deviceType === "Other" ? "" : "Intel(R) Arc(TM) Pro Graphics",
      backend: backend === "web_gl2" ? "Gl" : "BrowserWebGpu",
      device_type: deviceType,
      vendor: deviceType === "Other" ? 0 : 32902,
      device: 32085,
      driver: "",
      driver_info: "",
    },
  };
}

function phases(prefix) {
  const baselineAa = {
    intermediate_luma_pixels: 100,
    relative_hard_transitions: 100,
    normalized_squared_edge_energy: 100,
    luma_range: 200,
  };
  const fxaaAa = {
    intermediate_luma_pixels: 150,
    relative_hard_transitions: 50,
    normalized_squared_edge_energy: 80,
    luma_range: 200,
  };
  return {
    off: {
      id: "off", fnv1a64: `${prefix}-off`, nonblack: 100,
      aa_edge_metrics: baselineAa,
      resources_before_render: resources(1), resources_after_render: resources(1),
    },
    bloom_only: {
      id: "bloom_only", fnv1a64: `${prefix}-bloom`, nonblack: 120,
      aa_edge_metrics: baselineAa,
      resources_before_render: resources(11), resources_after_render: resources(11),
    },
    fxaa_only: {
      id: "fxaa_only", fnv1a64: `${prefix}-fxaa`, nonblack: 110,
      aa_edge_metrics: fxaaAa,
      resources_before_render: resources(11), resources_after_render: resources(11),
    },
    on: {
      id: "on", fnv1a64: `${prefix}-combined`, nonblack: 130,
      aa_edge_metrics: fxaaAa,
      resources_before_render: resources(11), resources_after_render: resources(11),
    },
    off_again: {
      id: "off_again", fnv1a64: `${prefix}-off`, nonblack: 100,
      aa_edge_metrics: baselineAa,
      resources_before_render: resources(1), resources_after_render: resources(1),
    },
  };
}

function validPf01() {
  return {
    schema: "scena.pf01.browser_output_toggle.v1",
    status: "passed",
    release_evidence: true,
    required_hardware: true,
    complete_backend_set: true,
    asset_url: "/assets/gltf/exploded_view_assembly.gltf",
    backends: ["webgpu", "webgl2"].map((backend) => ({
      backend,
      capability_report: capabilityReport(
        backend === "webgpu" ? "web_gpu" : "web_gl2",
        backend === "webgpu" ? "Other" : "IntegratedGpu",
      ),
      phases: phases(backend),
      browser_engine: "chromium",
      browser_gpu: browserGpu(),
      hardware_evidence: { status: "passed", failure_codes: [] },
    })),
  };
}

function validFr06Browser() {
  return {
    schema: "scena.fr06_semantic_aov_browser_proof.v1",
    required_hardware: true,
    complete_backend_set: true,
    release_evidence: true,
    backends: ["webgpu", "webgl2"].map((backend) => ({
      backend,
      hit_count: 500,
      finite_depth_count: 500,
      deterministic_repeat: true,
      capability_report: capabilityReport(
        backend === "webgpu" ? "web_gpu" : "web_gl2",
        backend === "webgpu" ? "Other" : "IntegratedGpu",
      ),
      browser_engine: "chromium",
      browser_gpu: browserGpu(),
      hardware_evidence: { status: "passed", failure_codes: [] },
    })),
    parity: {
      mask_agreement: 0.999,
      identity_agreement_on_common_hits: 1,
      max_depth_error_meters: 0.0005,
      min_normal_dot: 0.999,
      common_hits: 490,
    },
  };
}

function validNativeSurface() {
  const nativePhases = phases("native");
  for (const phase of Object.values(nativePhases)) {
    phase.prepared_resources = phase.resources_before_render;
    delete phase.resources_before_render;
    delete phase.resources_after_render;
  }
  return {
    schema: "scena.pf01_pf02.native_surface_hardware_proof.v1",
    status: "passed",
    release_evidence: true,
    surface_attached: true,
    backend: "Dx12",
    adapter: {
      name: "Intel(R) Arc(TM) Pro Graphics",
      backend: "Dx12",
      device_type: "IntegratedGpu",
      vendor: 32902,
      device: 32085,
      driver: "Intel",
      driver_info: "32.0.101.8517",
    },
    present_only: {
      readback_copies: 0,
      map_requests: 0,
      blocking_polls: 0,
      blocking_waits: 0,
      cpu_frame_copy_bytes: 0,
      gpu_buffer_creations: 0,
      gpu_texture_creations: 0,
      gpu_pipeline_creations: 0,
      gpu_bind_group_creations: 0,
      gpu_shader_module_creations: 0,
    },
    prepared_resources_before: resources(1),
    prepared_resources_after: resources(1),
    resize_lifecycle: {
      status: "passed",
      original_size: [320, 240],
      resized_size: [352, 256],
      target_changed_requires_prepare: true,
      rendered_after_resize: true,
      restored_original_size: true,
    },
    surface_loss_handling: {
      status: "passed",
      structured_surface_lost: true,
      host_surface_recreation_required: true,
      render_rejected_after_loss: true,
    },
    output_toggle: { status: "passed", phases: nativePhases },
    command: "scena-native-hardware-proof.exe",
  };
}

function validQ01Parity() {
  const png = Buffer.from([137, 80, 78, 71, 1]);
  const pngSha256 = crypto.createHash("sha256").update(png).digest("hex");
  return {
    schema: "scena.q01.required_webgpu_pixel_parity.v1",
    status: "passed",
    proof_class: "required-live-webgpu-pixel-parity",
    commit_sha: "0123456789abcdef0123456789abcdef01234567",
    timestamp_unix_seconds: 1,
    backend: "WebGpu",
    adapter: {
      name: "",
      backend: "BrowserWebGpu",
      device_type: "Other",
      vendor: 0,
      device: 0,
      driver: "",
      driver_info: "",
    },
    browser_gpu: browserGpu(),
    renderer_readback: { source: "renderer-owned-gpu-copy", width: 96, height: 96 },
    thresholds: {
      rgb_chebyshev_tolerance: 4,
      within_tolerance_fraction_min: 0.995,
      rgb_rmse_max: 2,
      p99_5_channel_delta_max: 4,
      foreground_iou_min: 0.995,
    },
    metrics: {
      compared_pixels: 8000,
      within_tolerance_fraction: 0.999,
      rgb_rmse: 0.5,
      p99_5_channel_delta: 2,
      foreground_iou: 0.999,
    },
    mutations: [
      "wrong-colors",
      "geometry-shift",
      "missing-object",
      "vertical-flip",
      "linear-as-srgb",
      "stale-reference",
    ].map((name) => ({ name, rejected: true })),
    images: ["cpu-reference", "gpu-live", "diff-heatmap"].map((kind) => ({
      kind,
      path: `target\\gate-artifacts\\m6-required-webgpu-pixel-parity\\${kind}.png`,
      sha256: pngSha256,
      bytes: png.length,
    })),
  };
}

function validQ04Lifecycle() {
  const baseline = {
    buffers: 10, gpu_textures: 20, render_targets: 4,
    pipelines: 9, bind_groups: 6, shader_modules: 3, pending_destructions: 0,
  };
  return {
    schema: "scena.q04.required_gpu_resource_lifecycle.v1",
    status: "passed",
    proof_class: "physical-hardware-required",
    commit_sha: "0123456789abcdef0123456789abcdef01234567",
    timestamp_unix_seconds: 1,
    source_checksums: [{
      path: "tests/c09_gpu_resource_lifecycle.rs",
      sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    }],
    adapter: validNativeSurface().adapter,
    baseline,
    prepared: {
      buffers: 12, gpu_textures: 27, render_targets: 11,
      pipelines: 21, bind_groups: 12, shader_modules: 10, pending_destructions: 0,
    },
    released: { ...baseline, pending_destructions: 72 },
    poll_status: "Confirmed",
    poll_pending_before: 72,
    poll_destroyed_resources: 72,
    poll_pending_after: 0,
    assertions_executed: 12,
    complete_lifecycle: true,
  };
}

function validP01Benchmark() {
  return {
    schema: "scena.p01.shader_module_cache.v1",
    status: "passed",
    commit_sha: "0123456789abcdef0123456789abcdef01234567",
    timestamp_unix_seconds: 1,
    command: "scena-p01-shader-module-cache.exe --exact full_gpu_reprepare_reuses_the_device_triangle_shader_module",
    adapter: validNativeSurface().adapter,
    hardware_adapter: true,
    sample_count: 5,
    cold_p95_ms: 20,
    warm_p95_ms: 10,
    p95_improvement_percent: 50,
    minimum_material_improvement_percent: 10,
    cold_shader_module_creations: 6,
    warm_shader_module_creations: 0,
    cold_triangle_shader_cache_misses: 1,
    warm_triangle_shader_cache_hits: 1,
  };
}

function validM8WaterBottle(pngSha256, diffSha256) {
  return {
    schema: "scena.m8.waterbottle_gpu_result.v1",
    status: "passed",
    release_evidence: true,
    commit_sha: "0123456789abcdef0123456789abcdef01234567",
    timestamp_unix_seconds: 1,
    backend: "Dx12",
    adapter: "Intel(R) Arc(TM) Pro Graphics (Dx12)",
    adapter_key: {
      schema: "scena.gpu_adapter_key.v1",
      backend: "Dx12",
      vendor: 32902,
      device: 32085,
      device_type: "IntegratedGpu",
      driver: "Intel",
      driver_info: "32.0.101.8517",
    },
    adapter_expectation: {
      schema: "scena.m8.waterbottle_adapter_expectation.v1",
      profile_id: "portable-physical-gpu-v1",
      match_key: null,
      owner: "scena-renderer-quality",
      reviewed_at: "2026-07-23",
      expires_at: "none",
      evidence_sha256: "4db449cdacf2340f8fa53937c28e5c4b5e2c7deaea73cbe0987dcd51eb93c751",
      regions: [
        { name: "cap_dome", x: 250, y: 70, expected: [76, 27, 12], tolerance: 25 },
        { name: "cap_dome_left", x: 240, y: 70, expected: [76, 27, 12], tolerance: 25 },
        { name: "upper_body", x: 249, y: 130, expected: [153, 134, 48], tolerance: 25 },
        { name: "body_olive_mid", x: 249, y: 270, expected: [163, 143, 53], tolerance: 25 },
        { name: "body_olive_low", x: 249, y: 330, expected: [132, 114, 32], tolerance: 25 },
        { name: "label_metal_r", x: 270, y: 380, expected: [30, 20, 6], tolerance: 25 },
        { name: "label_metal_l", x: 255, y: 380, expected: [28, 18, 5], tolerance: 25 },
      ],
    },
    png_sha256: pngSha256,
    diff_sha256: diffSha256,
    reference_sha256: "4db449cdacf2340f8fa53937c28e5c4b5e2c7deaea73cbe0987dcd51eb93c751",
    metrics: {
      reference_diff: "passed",
      full_frame: {
        compared_pixels: 512 * 512,
        within_tolerance_fraction: 0.99,
        max_channel_delta: 24,
        worst_region_bbox: [10, 20, 40, 60],
      },
      thresholds: {
        rgb_chebyshev_max: 16,
        within_tolerance_fraction_min: 0.95,
      },
    },
    known_bad_mutations: [{ name: "horizontal_mirror", rejected: true }],
  };
}

function validQ07Antialiasing() {
  const baseline = {
    intermediate_luma_pixels: 4_554,
    hard_transition_count: 762,
    squared_edge_energy: 1_000_000,
    luma_range: 200,
  };
  const candidate = (intermediate, hard, energy) => ({
    intermediate_luma_pixels: intermediate,
    hard_transition_count: hard,
    squared_edge_energy: energy,
    luma_range: 190,
  });
  return {
    schema: "scena.q07.antialiasing_effect.v1",
    status: "passed",
    release_evidence: true,
    commit_sha: "0123456789abcdef0123456789abcdef01234567",
    timestamp_unix_seconds: 1,
    fixture: "high-contrast-asymmetric-diagonal-v1",
    adapter: validNativeSurface().adapter,
    baseline: { mode: "none", metrics: baseline },
    modes: {
      fxaa: { status: "passed", metrics: candidate(4_831, 500, 700_000) },
      msaa4: { status: "passed", metrics: candidate(4_800, 550, 800_000) },
      msaa8: {
        status: "degraded",
        reason_code: "UNSUPPORTED_SAMPLE_COUNT",
        requested: 8,
        maximum: 4,
      },
    },
    known_bad_mutations: [
      { name: "no_op", rejected: true },
      { name: "blur_everything", rejected: true },
    ],
  };
}

const Q08_FIXTURES = [
  ["physical-glass-transmission-matches-cpu-and-gpu-across-volume-sweep.json", "physical_glass_transmission_matches_cpu_and_gpu_across_volume_sweep", "tests/transmission_parity.rs"],
  ["close-camera-near-clip-matches-cpu-and-gpu-rendered-output.json", "close_camera_near_clip_matches_cpu_and_gpu_rendered_output", "tests/c13_depth_clipping_parity.rs"],
  ["dynamic-transform-motion-matches-cpu-and-gpu-for-authored-animation-and-imports.json", "dynamic_transform_motion_matches_cpu_and_gpu_for_authored_animation_and_imports", "tests/dynamic_transform_parity.rs"],
  ["z-up-imported-rotation-frame-matches-cpu-and-gpu-after-basis-conversion.json", "z_up_imported_rotation_frame_matches_cpu_and_gpu_after_basis_conversion", "tests/dynamic_transform_parity.rs"],
  ["core-pbr-brdf-matches-cpu-and-gpu-across-metallic-roughness-sweep.json", "core_pbr_brdf_matches_cpu_and_gpu_across_metallic_roughness_sweep", "tests/pbr_brdf_parity.rs"],
  ["pf08-adaptive-texture-bake-preserves-seams-perspective-and-material-identity-cpu-gpu.json", "pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu", "tests/pf08_texture_bake_parity.rs"],
];

function validQ08Parity(testName, source, sourceSha256) {
  return {
    schema: "scena.q08.required_cpu_gpu_parity.v1",
    status: "passed",
    release_evidence: true,
    proof_class: "physical-hardware-required",
    test_name: testName,
    producer: `cargo test ${testName} -- --exact`,
    commit_sha: "0123456789abcdef0123456789abcdef01234567",
    timestamp_unix_seconds: 1,
    assertions_executed: 8,
    adapter: validNativeSurface().adapter,
    source_checksums: [{ path: source, sha256: sourceSha256 }],
  };
}

function validQ11ReferenceStability() {
  const renderSha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
  const metrics = {
    passed: true,
    within_tolerance_fraction: 1,
    rgb_rmse: 0,
    alpha_mismatch_pixels: 0,
  };
  return {
    schema: "scena.q11.reference_stability.v1",
    status: "passed",
    release_evidence: true,
    test_name: "q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison",
    commit_sha: "0123456789abcdef0123456789abcdef01234567",
    timestamp_unix_seconds: 1,
    os: "windows",
    arch: "x86_64",
    backend: "Headless",
    adapter: "software-rasterizer",
    comparison_order: "independent-render-before-committed-reference",
    repeat_count: 2,
    byte_identical: true,
    rgba8_sha256: [renderSha, renderSha],
    metric_distribution: [metrics, metrics],
    reference: {
      sha256: "8bbaa66e23a3dea4a9efcb53f9157226b666cd77bd020dea30cd89277d3037b5",
      rgb_chebyshev_tolerance: 4,
      min_within_tolerance_fraction: 0.995,
      max_rgb_rmse: 2,
    },
  };
}

function validNativeFr06() {
  const nativeCapabilityReport = capabilityReport("headless_gpu");
  nativeCapabilityReport.adapter = { ...validNativeSurface().adapter };
  return {
    schema: "scena.fr06.native_semantic_aov_proof.v1",
    status: "passed",
    release_evidence: true,
    required_hardware: true,
    capability_report: nativeCapabilityReport,
    coverage: { gpu_hit_count: 100, gpu_finite_depth_count: 100 },
    center: {
      cpu_id: 1,
      gpu_id: 1,
      cpu_depth_meters: 0.95,
      gpu_depth_meters: 0.9505,
      cpu_world_normal: [0, 0, 1],
      gpu_world_normal: [0.001, -0.001, 0.999],
    },
    tolerances: {
      max_depth_error_meters: 0.001,
      max_normal_component_error: 0.01,
    },
    command: "scena-fr06-native-hardware-proof.exe --exact fr06_headless_gpu_semantic_aov_matches_cpu_center_truth",
  };
}

function writeJson(root, relative, value) {
  const destination = path.join(root, relative);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, `${JSON.stringify(value, null, 2)}\n`);
}

function createProofRoot() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "scena-windows-proof-validator-"));
  writeJson(root, "target/gate-artifacts/pf01-output-toggle/browser/browser-output-toggle.json", validPf01());
  writeJson(root, "target/gate-artifacts/fr06-semantic-aov/browser/semantic-aov-browser-proof.json", validFr06Browser());
  writeJson(root, "target/gate-artifacts/pf01-pf02-native-surface/native-present-only.json", validNativeSurface());
  writeJson(root, "target/gate-artifacts/fr06-semantic-aov/native/native-semantic-aov-proof.json", validNativeFr06());
  writeJson(root, "target/gate-artifacts/m6-required-webgpu-pixel-parity/result.json", validQ01Parity());
  writeJson(root, "target/gate-artifacts/c09-gpu-resource-lifecycle/required-result.json", validQ04Lifecycle());
  writeJson(root, "target/gate-artifacts/p01-shader-module-cache.json", validP01Benchmark());
  const m8Png = path.join(root, "target/gate-artifacts/m8-real-asset/waterbottle_gpu.png");
  const m8Diff = path.join(root, "target/gate-artifacts/m8-real-asset/waterbottle_diff.png");
  fs.mkdirSync(path.dirname(m8Png), { recursive: true });
  fs.writeFileSync(m8Png, Buffer.from([137, 80, 78, 71, 1]));
  fs.writeFileSync(m8Diff, Buffer.from([137, 80, 78, 71, 2]));
  writeJson(
    root,
    "target/gate-artifacts/m8-real-asset/waterbottle_gpu_result.json",
    validM8WaterBottle(
      crypto.createHash("sha256").update(fs.readFileSync(m8Png)).digest("hex"),
      crypto.createHash("sha256").update(fs.readFileSync(m8Diff)).digest("hex"),
    ),
  );
  writeJson(
    root,
    "target/gate-artifacts/q07-antialiasing-effect/result.json",
    validQ07Antialiasing(),
  );
  writeJson(
    root,
    "target/gate-artifacts/q11-reference-stability/windows-x86_64.json",
    validQ11ReferenceStability(),
  );
  const sourceHashes = new Map();
  for (const [, , source] of Q08_FIXTURES) {
    if (!sourceHashes.has(source)) {
      const sourcePath = path.join(root, source);
      fs.mkdirSync(path.dirname(sourcePath), { recursive: true });
      fs.writeFileSync(sourcePath, `Q08 fixture source: ${source}\n`);
      sourceHashes.set(source, crypto.createHash("sha256").update(fs.readFileSync(sourcePath)).digest("hex"));
    }
  }
  for (const [name, testName, source] of Q08_FIXTURES) {
    writeJson(
      root,
      `target/gate-artifacts/q08-required-parity/${name}`,
      validQ08Parity(testName, source, sourceHashes.get(source)),
    );
  }
  for (const backend of ["webgpu", "webgl2"]) {
    const png = path.join(root, `target/gate-artifacts/fr06-semantic-aov/browser/${backend}-id.png`);
    fs.writeFileSync(png, Buffer.from([137, 80, 78, 71, 1]));
  }
  for (const phase of ["off", "bloom-only", "fxaa-only", "on", "off-again"]) {
    const ppm = path.join(root, `target/gate-artifacts/pf01-pf02-native-surface/${phase}.ppm`);
    fs.writeFileSync(ppm, Buffer.from("P6\n1 1\n255\n\x01\x02\x03", "binary"));
  }
  for (const image of ["cpu-reference", "gpu-live", "diff-heatmap"]) {
    const png = path.join(root, `target/gate-artifacts/m6-required-webgpu-pixel-parity/${image}.png`);
    fs.writeFileSync(png, Buffer.from([137, 80, 78, 71, 1]));
  }
  for (const mode of ["none", "fxaa", "msaa4"]) {
    const ppm = path.join(root, `target/gate-artifacts/q07-antialiasing-effect/${mode}.ppm`);
    fs.mkdirSync(path.dirname(ppm), { recursive: true });
    fs.writeFileSync(ppm, Buffer.from("P6\n1 1\n255\n\x01\x02\x03", "binary"));
  }
  return root;
}

function mutate(root, relative, callback) {
  const file = path.join(root, relative);
  const value = JSON.parse(fs.readFileSync(file, "utf8"));
  callback(value);
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function expectFailure(callback, pattern) {
  assert.throws(callback, pattern);
}

{
  const root = createProofRoot();
  const summary = validateProofRoot(root);
  assert.strictEqual(summary.schema, "scena.windows_complete_hardware_proof.v1");
  assert.strictEqual(summary.status, "passed");
  assert.strictEqual(summary.hardware_evidence, true);
  assert.strictEqual(summary.release_evidence, false);
  assert.deepStrictEqual(summary.release_provenance, {
    status: "unavailable",
    reason: "the collected hardware artifacts are not bound to one exact source commit",
  });
  assert.deepStrictEqual(summary.coverage, {
    browser_backends: ["webgl2", "webgpu"],
    native_surface: true,
    native_semantic_aov: true,
    required_webgpu_pixel_parity: true,
    gpu_resource_lifecycle: true,
    shader_module_cache_p95: true,
    waterbottle_full_frame_reference: true,
    waterbottle_horizontal_mirror_rejected: true,
    antialiasing_pixel_effect: true,
    reference_stability: true,
    required_physical_cpu_gpu_parity: true,
    native_surface_resize_recovery: true,
  });
  assert.strictEqual(Object.keys(summary.artifact_sha256).length, 31);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/q11-reference-stability/windows-x86_64.json", (report) => {
    report.byte_identical = false;
  });
  expectFailure(() => validateProofRoot(root), /Q11.*independent renders/i);
}

{
  const root = createProofRoot();
  mutate(
    root,
    "target/gate-artifacts/q08-required-parity/physical-glass-transmission-matches-cpu-and-gpu-across-volume-sweep.json",
    (report) => {
      report.assertions_executed = 0;
    },
  );
  expectFailure(() => validateProofRoot(root), /Q08 parity.*executed zero assertions/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/m8-real-asset/waterbottle_gpu_result.json", (report) => {
    report.known_bad_mutations[0].rejected = false;
  });
  expectFailure(() => validateProofRoot(root), /M8.*horizontal-mirror.*not rejected/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/m8-real-asset/waterbottle_gpu_result.json", (report) => {
    report.adapter_expectation.profile_id = "display-name-selected-profile";
  });
  expectFailure(() => validateProofRoot(root), /M8.*adapter expectation.*structured portable profile/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/q07-antialiasing-effect/result.json", (report) => {
    report.modes.msaa4.metrics = { ...report.baseline.metrics };
  });
  expectFailure(() => validateProofRoot(root), /Q07 msaa4.*no measured AA pixel effect/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/q07-antialiasing-effect/result.json", (report) => {
    report.adapter.backend = "Vulkan";
  });
  expectFailure(() => validateProofRoot(root), /Q07 AA proof.*must use Dx12/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/m6-required-webgpu-pixel-parity/result.json", (report) => {
    report.mutations[0].rejected = false;
  });
  expectFailure(() => validateProofRoot(root), /Q01.*mutation.*wrong-colors/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/m6-required-webgpu-pixel-parity/result.json", (report) => {
    report.browser_gpu.devices = [{
      vendor_id: 0x1ae0,
      device_id: 0xc0de,
      vendor_string: "Google",
      device_string: "Google SwiftShader",
    }];
    report.browser_gpu.aux_attributes = {
      gl_vendor: "Google",
      gl_renderer: "ANGLE (Google, Vulkan SwiftShader Device)",
    };
  });
  expectFailure(() => validateProofRoot(root), /Q01.*SOFTWARE_ADAPTER/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/c09-gpu-resource-lifecycle/required-result.json", (report) => {
    report.source_checksums = [];
  });
  expectFailure(() => validateProofRoot(root), /Q04.*source checksums/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/c09-gpu-resource-lifecycle/required-result.json", (report) => {
    report.poll_pending_after = 1;
  });
  expectFailure(() => validateProofRoot(root), /Q04.*pending destructions/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/p01-shader-module-cache.json", (report) => {
    report.p95_improvement_percent = 2;
  });
  expectFailure(() => validateProofRoot(root), /P01.*p95 improvement/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/pf01-pf02-native-surface/native-present-only.json", (report) => {
    report.resize_lifecycle.rendered_after_resize = false;
  });
  expectFailure(() => validateProofRoot(root), /resize lifecycle/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/pf01-output-toggle/browser/browser-output-toggle.json", (report) => {
    report.backends[0].phases.on.fnv1a64 = report.backends[0].phases.fxaa_only.fnv1a64;
  });
  expectFailure(() => validateProofRoot(root), /combined output.*FXAA-only/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/pf01-pf02-native-surface/native-present-only.json", (report) => {
    report.present_only.map_requests = 1;
  });
  expectFailure(() => validateProofRoot(root), /map_requests.*zero/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/fr06-semantic-aov/browser/semantic-aov-browser-proof.json", (report) => {
    report.parity.mask_agreement = 0.5;
  });
  expectFailure(() => validateProofRoot(root), /mask agreement/i);
}

{
  const root = createProofRoot();
  mutate(root, "target/gate-artifacts/fr06-semantic-aov/native/native-semantic-aov-proof.json", (report) => {
    report.release_evidence = false;
  });
  expectFailure(() => validateProofRoot(root), /native FR06.*release evidence/i);
}

{
  const root = createProofRoot();
  fs.rmSync(path.join(root, "target/gate-artifacts/pf01-pf02-native-surface/on.ppm"));
  expectFailure(() => validateProofRoot(root), /missing visual artifact.*on\.ppm/i);
}

console.log("Windows complete hardware proof validator: pass");
