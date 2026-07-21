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
  return {
    off: {
      id: "off", fnv1a64: `${prefix}-off`, nonblack: 100,
      resources_before_render: resources(1), resources_after_render: resources(1),
    },
    bloom_only: {
      id: "bloom_only", fnv1a64: `${prefix}-bloom`, nonblack: 120,
      resources_before_render: resources(11), resources_after_render: resources(11),
    },
    fxaa_only: {
      id: "fxaa_only", fnv1a64: `${prefix}-fxaa`, nonblack: 110,
      resources_before_render: resources(11), resources_after_render: resources(11),
    },
    on: {
      id: "on", fnv1a64: `${prefix}-combined`, nonblack: 130,
      resources_before_render: resources(11), resources_after_render: resources(11),
    },
    off_again: {
      id: "off_again", fnv1a64: `${prefix}-off`, nonblack: 100,
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
    adapter: {
      name: "Intel(R) Arc(TM) Pro Graphics",
      backend: "BrowserWebGpu",
      device_type: "IntegratedGpu",
      vendor: 32902,
      device: 32085,
      driver: "Intel",
      driver_info: "32.0.101.8517",
    },
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
      path: `target/gate-artifacts/m6-required-webgpu-pixel-parity/${kind}.png`,
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

function validNativeFr06() {
  return {
    schema: "scena.fr06.native_semantic_aov_proof.v1",
    status: "passed",
    release_evidence: true,
    required_hardware: true,
    capability_report: capabilityReport("vulkan"),
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
    native_surface_resize_recovery: true,
  });
  assert.strictEqual(Object.keys(summary.artifact_sha256).length, 17);
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
