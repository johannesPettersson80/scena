const crypto = require("crypto");
const fs = require("fs");
const path = require("path");

const { validateOutputToggleResult } = require("../browser/pf01_output_toggle_validation.js");
const {
  evaluateRequiredHardwareAdapter,
} = require("../browser/required_gpu_parity.js");

const JSON_ARTIFACTS = {
  pf01: "target/gate-artifacts/pf01-output-toggle/browser/browser-output-toggle.json",
  fr06Browser: "target/gate-artifacts/fr06-semantic-aov/browser/semantic-aov-browser-proof.json",
  nativeSurface: "target/gate-artifacts/pf01-pf02-native-surface/native-present-only.json",
  nativeFr06: "target/gate-artifacts/fr06-semantic-aov/native/native-semantic-aov-proof.json",
  q01Parity: "target/gate-artifacts/m6-required-webgpu-pixel-parity/result.json",
  q04Lifecycle: "target/gate-artifacts/c09-gpu-resource-lifecycle/required-result.json",
  p01Benchmark: "target/gate-artifacts/p01-shader-module-cache.json",
};

const VISUAL_ARTIFACTS = [
  "target/gate-artifacts/fr06-semantic-aov/browser/webgpu-id.png",
  "target/gate-artifacts/fr06-semantic-aov/browser/webgl2-id.png",
  "target/gate-artifacts/pf01-pf02-native-surface/off.ppm",
  "target/gate-artifacts/pf01-pf02-native-surface/bloom-only.ppm",
  "target/gate-artifacts/pf01-pf02-native-surface/fxaa-only.ppm",
  "target/gate-artifacts/pf01-pf02-native-surface/on.ppm",
  "target/gate-artifacts/pf01-pf02-native-surface/off-again.ppm",
  "target/gate-artifacts/m6-required-webgpu-pixel-parity/cpu-reference.png",
  "target/gate-artifacts/m6-required-webgpu-pixel-parity/gpu-live.png",
  "target/gate-artifacts/m6-required-webgpu-pixel-parity/diff-heatmap.png",
];

const SOFTWARE_MARKERS = [
  "swiftshader",
  "llvmpipe",
  "lavapipe",
  "software rasterizer",
  "microsoft basic render",
];

const PHASE_IDS = ["off", "bloom_only", "fxaa_only", "on", "off_again"];
const Q01_MUTATIONS = [
  "wrong-colors",
  "geometry-shift",
  "missing-object",
  "vertical-flip",
  "linear-as-srgb",
  "stale-reference",
];

function invariant(condition, message) {
  if (!condition) throw new Error(message);
}

function readJson(root, relative) {
  const file = path.join(root, relative);
  invariant(fs.existsSync(file), `missing JSON artifact: ${relative}`);
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function normalizedBackend(value) {
  return String(value || "").toLowerCase().replace(/[^a-z0-9]/g, "");
}

function assertBackendSet(backends, label) {
  invariant(Array.isArray(backends) && backends.length === 2, `${label} must contain exactly two backends`);
  const names = backends.map((backend) => String(backend.backend || "")).sort();
  invariant(names.join(",") === "webgl2,webgpu", `${label} backend set is invalid: ${names.join(",")}`);
}

function assertBrowserHardware(backend, label) {
  invariant(backend.browser_engine === "chromium", `${label} did not use Chromium`);
  invariant(
    backend.browser_gpu && backend.browser_gpu.source === "chromium-cdp-system-info",
    `${label} is missing Chromium CDP GPU evidence`,
  );
  invariant(
    backend.hardware_evidence && backend.hardware_evidence.status === "passed",
    `${label} stored hardware evidence did not pass`,
  );
  invariant(
    Array.isArray(backend.hardware_evidence.failure_codes)
      && backend.hardware_evidence.failure_codes.length === 0,
    `${label} stored hardware evidence has failure codes`,
  );
  const report = backend.capability_report || {};
  const capabilities = report.capabilities || {};
  const evaluated = evaluateRequiredHardwareAdapter({
    required: true,
    requestedBackend: backend.backend,
    actualBackend: capabilities.backend,
    gpuDevice: capabilities.gpu_device,
    surfaceAttached: capabilities.surface_attached,
    adapter: report.adapter,
    browserGpu: backend.browser_gpu,
  });
  invariant(
    evaluated.status === "passed",
    `${label} independently evaluated hardware evidence failed: ${evaluated.failure_codes.join(",")}`,
  );
}

function assertNativeHardware(adapter, label) {
  invariant(adapter && typeof adapter === "object", `${label} has no adapter report`);
  invariant(
    ["DiscreteGpu", "IntegratedGpu", "VirtualGpu"].includes(String(adapter.device_type || "")),
    `${label} adapter is not a hardware device type: ${adapter.device_type}`,
  );
  const identity = [adapter.name, adapter.device_type, adapter.driver, adapter.driver_info]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
  for (const marker of SOFTWARE_MARKERS) {
    invariant(!identity.includes(marker), `${label} uses a software adapter marker: ${marker}`);
  }
}

function assertProvenance(report, label) {
  invariant(
    /^[0-9a-f]{40}$/i.test(String(report.commit_sha || "")),
    `${label} has no exact source commit`,
  );
  invariant(
    Number.isInteger(report.timestamp_unix_seconds) && report.timestamp_unix_seconds > 0,
    `${label} has no positive timestamp`,
  );
}

function validatePf01(report) {
  invariant(report.schema === "scena.pf01.browser_output_toggle.v1", "unexpected PF01 browser schema");
  invariant(report.status === "passed", "PF01 browser status is not passed");
  invariant(report.release_evidence === true, "PF01 browser artifact is not release evidence");
  invariant(report.required_hardware === true, "PF01 browser artifact did not require hardware");
  invariant(report.complete_backend_set === true, "PF01 browser artifact is incomplete");
  assertBackendSet(report.backends, "PF01 browser artifact");
  for (const backend of report.backends) {
    assertBrowserHardware(backend, `PF01 ${backend.backend}`);
    validateOutputToggleResult(backend);
  }
  return report;
}

function validateFr06Browser(report) {
  invariant(
    report.schema === "scena.fr06_semantic_aov_browser_proof.v1",
    "unexpected browser FR06 schema",
  );
  invariant(report.required_hardware === true, "browser FR06 did not require hardware");
  invariant(report.complete_backend_set === true, "browser FR06 backend set is incomplete");
  invariant(report.release_evidence === true, "browser FR06 is not release evidence");
  assertBackendSet(report.backends, "browser FR06 artifact");
  for (const backend of report.backends) {
    assertBrowserHardware(backend, `FR06 ${backend.backend}`);
    invariant(Number.isInteger(backend.hit_count) && backend.hit_count > 0, `FR06 ${backend.backend} has no hits`);
    invariant(
      backend.finite_depth_count === backend.hit_count,
      `FR06 ${backend.backend} does not have finite depth for every hit`,
    );
    invariant(backend.deterministic_repeat === true, `FR06 ${backend.backend} repeat capture is not deterministic`);
  }
  const parity = report.parity || {};
  invariant(Number(parity.common_hits) > 0, "FR06 browser parity has no common hits");
  invariant(Number(parity.mask_agreement) >= 0.98, "FR06 browser mask agreement is below 0.98");
  invariant(
    Number(parity.identity_agreement_on_common_hits) >= 0.995,
    "FR06 browser identity agreement is below 0.995",
  );
  invariant(
    Number(parity.max_depth_error_meters) <= 0.005,
    "FR06 browser depth error exceeds 0.005 meters",
  );
  invariant(Number(parity.min_normal_dot) >= 0.98, "FR06 browser normal agreement is below 0.98");
  return report;
}

function assertNativePhaseSet(phases) {
  invariant(phases && typeof phases === "object", "native output toggle has no phases");
  for (const id of PHASE_IDS) {
    const phase = phases[id];
    invariant(phase && phase.id === id, `native output toggle is missing phase ${id}`);
    invariant(Number.isInteger(phase.nonblack) && phase.nonblack > 0, `native ${id} output is blank`);
    invariant(typeof phase.fnv1a64 === "string" && phase.fnv1a64.length > 0, `native ${id} has no pixel hash`);
    invariant(Array.isArray(phase.prepared_resources), `native ${id} has no resource signature`);
  }
  const { off, bloom_only: bloom, fxaa_only: fxaa, on, off_again: offAgain } = phases;
  invariant(bloom.fnv1a64 !== off.fnv1a64, "native bloom-only output is identical to baseline");
  invariant(fxaa.fnv1a64 !== off.fnv1a64, "native FXAA-only output is identical to baseline");
  invariant(on.fnv1a64 !== off.fnv1a64, "native combined output is identical to baseline");
  invariant(on.fnv1a64 !== bloom.fnv1a64, "native combined output is identical to bloom-only");
  invariant(on.fnv1a64 !== fxaa.fnv1a64, "native combined output is identical to FXAA-only");
  invariant(offAgain.fnv1a64 === off.fnv1a64, "native off-again pixels do not restore baseline");
  const signature = (value) => JSON.stringify(value);
  for (const phase of [bloom, fxaa, on]) {
    invariant(
      signature(phase.prepared_resources) !== signature(off.prepared_resources),
      `native ${phase.id} did not prepare a distinct resource shape`,
    );
  }
  invariant(
    signature(offAgain.prepared_resources) === signature(off.prepared_resources),
    "native off-again resources do not restore baseline",
  );
}

function validateNativeSurface(report) {
  invariant(
    report.schema === "scena.pf01_pf02.native_surface_hardware_proof.v1",
    "unexpected native surface schema",
  );
  invariant(report.status === "passed", "native surface status is not passed");
  invariant(report.release_evidence === true, "native surface artifact is not release evidence");
  invariant(report.surface_attached === true, "native surface artifact was not attached");
  invariant(normalizedBackend(report.backend).length > 0, "native surface artifact has no backend");
  assertNativeHardware(report.adapter, "native surface");
  const presentOnly = report.present_only || {};
  for (const counter of [
    "readback_copies",
    "map_requests",
    "blocking_polls",
    "blocking_waits",
    "cpu_frame_copy_bytes",
    "gpu_buffer_creations",
    "gpu_texture_creations",
    "gpu_pipeline_creations",
    "gpu_bind_group_creations",
    "gpu_shader_module_creations",
  ]) {
    invariant(Number(presentOnly[counter]) === 0, `native present-only ${counter} must be zero`);
  }
  invariant(
    JSON.stringify(report.prepared_resources_before) === JSON.stringify(report.prepared_resources_after),
    "native present-only render changed prepared resources",
  );
  invariant(report.output_toggle && report.output_toggle.status === "passed", "native output-toggle status is not passed");
  const resize = report.resize_lifecycle || {};
  invariant(
    resize.status === "passed"
      && Array.isArray(resize.original_size)
      && Array.isArray(resize.resized_size)
      && resize.target_changed_requires_prepare === true
      && resize.rendered_after_resize === true
      && resize.restored_original_size === true,
    "native attached-surface resize lifecycle did not pass",
  );
  const recovery = report.surface_loss_handling || {};
  invariant(
    recovery.status === "passed"
      && recovery.structured_surface_lost === true
      && recovery.host_surface_recreation_required === true
      && recovery.render_rejected_after_loss === true,
    "native attached-surface loss handling did not pass",
  );
  assertNativePhaseSet(report.output_toggle.phases);
  invariant(
    String(report.command || "").includes("scena-native-hardware-proof"),
    "native surface artifact does not identify the proof executable",
  );
  return report;
}

function validateQ01Parity(report, root) {
  invariant(report.schema === "scena.q01.required_webgpu_pixel_parity.v1", "unexpected Q01 parity schema");
  invariant(report.status === "passed", "Q01 parity status is not passed");
  invariant(
    report.proof_class === "required-live-webgpu-pixel-parity",
    "Q01 parity is not the required live proof class",
  );
  assertProvenance(report, "Q01 parity");
  assertNativeHardware(report.adapter, "Q01 WebGPU parity");
  invariant(
    report.renderer_readback
      && report.renderer_readback.source === "renderer-owned-gpu-copy"
      && Number(report.renderer_readback.width) > 0
      && Number(report.renderer_readback.height) > 0,
    "Q01 parity did not evaluate a renderer-owned readback",
  );
  const thresholds = report.thresholds || {};
  invariant(
    thresholds.rgb_chebyshev_tolerance === 4
      && thresholds.within_tolerance_fraction_min === 0.995
      && thresholds.rgb_rmse_max === 2
      && thresholds.p99_5_channel_delta_max === 4
      && thresholds.foreground_iou_min === 0.995,
    "Q01 parity thresholds are incomplete or changed",
  );
  const metrics = report.metrics || {};
  invariant(
    Number(metrics.compared_pixels) > 0
      && Number(metrics.within_tolerance_fraction) >= thresholds.within_tolerance_fraction_min
      && Number(metrics.rgb_rmse) <= thresholds.rgb_rmse_max
      && Number(metrics.p99_5_channel_delta) <= thresholds.p99_5_channel_delta_max
      && Number(metrics.foreground_iou) >= thresholds.foreground_iou_min,
    "Q01 parity metrics do not pass the declared thresholds",
  );
  for (const name of Q01_MUTATIONS) {
    invariant(
      Array.isArray(report.mutations)
        && report.mutations.some((mutation) => mutation.name === name && mutation.rejected === true),
      `Q01 parity mutation ${name} was not rejected`,
    );
  }
  for (const kind of ["cpu-reference", "gpu-live", "diff-heatmap"]) {
    const image = Array.isArray(report.images)
      ? report.images.find((candidate) => candidate.kind === kind)
      : null;
    invariant(image, `Q01 parity is missing ${kind} image metadata`);
    const expected = `target/gate-artifacts/m6-required-webgpu-pixel-parity/${kind}.png`;
    invariant(image.path === expected, `Q01 parity ${kind} image path is not canonical`);
    const file = path.join(root, expected);
    invariant(fs.existsSync(file), `Q01 parity is missing ${kind} image`);
    invariant(image.sha256 === artifactHash(file), `Q01 parity ${kind} image checksum does not match`);
    invariant(Number(image.bytes) === fs.statSync(file).size, `Q01 parity ${kind} image byte count does not match`);
  }
  return report;
}

function lifecycleShape(value) {
  return ["buffers", "gpu_textures", "render_targets", "pipelines", "bind_groups", "shader_modules"]
    .map((field) => Number(value && value[field]));
}

function validateQ04Lifecycle(report) {
  invariant(report.schema === "scena.q04.required_gpu_resource_lifecycle.v1", "unexpected Q04 lifecycle schema");
  invariant(
    report.status === "passed" && report.proof_class === "physical-hardware-required",
    "Q04 lifecycle is not a passed physical-hardware proof",
  );
  assertProvenance(report, "Q04 lifecycle");
  assertNativeHardware(report.adapter, "Q04 lifecycle");
  invariant(
    report.complete_lifecycle === true && Number(report.assertions_executed) >= 10,
    "Q04 lifecycle did not execute its complete assertion set",
  );
  const baseline = lifecycleShape(report.baseline);
  const prepared = lifecycleShape(report.prepared);
  const released = lifecycleShape(report.released);
  invariant(
    prepared.every(Number.isFinite)
      && baseline.every(Number.isFinite)
      && prepared.reduce((sum, value) => sum + value, 0) > baseline.reduce((sum, value) => sum + value, 0),
    "Q04 lifecycle did not prepare additional tracked resources",
  );
  invariant(
    JSON.stringify(released) === JSON.stringify(baseline),
    "Q04 lifecycle did not restore the baseline retained resource shape",
  );
  invariant(
    Number(report.released && report.released.pending_destructions) > 0
      && Number(report.poll_pending_before) === Number(report.released.pending_destructions)
      && Number(report.poll_destroyed_resources) === Number(report.poll_pending_before)
      && Number(report.poll_pending_after) === Number(report.baseline && report.baseline.pending_destructions)
      && report.poll_status === "Confirmed",
    "Q04 pending destructions were not confirmed and retired",
  );
  return report;
}

function validateP01Benchmark(report) {
  invariant(report.schema === "scena.p01.shader_module_cache.v1", "unexpected P01 benchmark schema");
  invariant(report.status === "passed" && report.hardware_adapter === true, "P01 benchmark is not a passed hardware run");
  assertProvenance(report, "P01 benchmark");
  assertNativeHardware(report.adapter, "P01 benchmark");
  invariant(Number(report.sample_count) >= 5, "P01 benchmark has too few samples");
  invariant(
    Number(report.cold_shader_module_creations) > Number(report.warm_shader_module_creations)
      && Number(report.cold_triangle_shader_cache_misses) > 0
      && Number(report.warm_triangle_shader_cache_hits) > 0,
    "P01 benchmark did not prove shader-module cache reuse",
  );
  invariant(
    Number(report.p95_improvement_percent) >= Number(report.minimum_material_improvement_percent)
      && Number(report.warm_p95_ms) < Number(report.cold_p95_ms),
    "P01 hardware p95 improvement is below the declared material threshold",
  );
  invariant(
    String(report.command || "").includes("scena-p01-shader-module-cache"),
    "P01 benchmark does not identify the proof executable",
  );
  return report;
}

function validateNativeFr06(report) {
  invariant(report.schema === "scena.fr06.native_semantic_aov_proof.v1", "unexpected native FR06 schema");
  invariant(report.status === "passed", "native FR06 status is not passed");
  invariant(report.release_evidence === true, "native FR06 artifact is not release evidence");
  invariant(report.required_hardware === true, "native FR06 did not require hardware");
  const capabilityReport = report.capability_report || {};
  invariant(
    capabilityReport.capabilities && capabilityReport.capabilities.gpu_device === true,
    "native FR06 capability report has no GPU device",
  );
  assertNativeHardware(capabilityReport.adapter, "native FR06");
  const coverage = report.coverage || {};
  invariant(Number(coverage.gpu_hit_count) > 0, "native FR06 has no GPU hits");
  invariant(
    Number(coverage.gpu_finite_depth_count) === Number(coverage.gpu_hit_count),
    "native FR06 does not have finite depth for every hit",
  );
  const center = report.center || {};
  invariant(Number(center.gpu_id) !== 0, "native FR06 center is not attributed");
  invariant(center.gpu_id === center.cpu_id, "native FR06 CPU/GPU center IDs differ");
  invariant(Number.isFinite(center.cpu_depth_meters), "native FR06 CPU center depth is not finite");
  invariant(Number.isFinite(center.gpu_depth_meters), "native FR06 GPU center depth is not finite");
  const depthTolerance = Number(report.tolerances && report.tolerances.max_depth_error_meters);
  invariant(Number.isFinite(depthTolerance) && depthTolerance > 0, "native FR06 depth tolerance is invalid");
  invariant(
    Math.abs(center.gpu_depth_meters - center.cpu_depth_meters) <= depthTolerance,
    "native FR06 center depth exceeds tolerance",
  );
  const normalTolerance = Number(report.tolerances && report.tolerances.max_normal_component_error);
  invariant(Number.isFinite(normalTolerance) && normalTolerance > 0, "native FR06 normal tolerance is invalid");
  invariant(
    Array.isArray(center.cpu_world_normal)
      && Array.isArray(center.gpu_world_normal)
      && center.cpu_world_normal.length === 3
      && center.gpu_world_normal.length === 3,
    "native FR06 center normals are invalid",
  );
  for (let component = 0; component < 3; component += 1) {
    invariant(
      Math.abs(center.gpu_world_normal[component] - center.cpu_world_normal[component]) <= normalTolerance,
      `native FR06 normal component ${component} exceeds tolerance`,
    );
  }
  invariant(
    String(report.command || "").includes("scena-fr06-native-hardware-proof"),
    "native FR06 artifact does not identify the proof executable",
  );
  return report;
}

function artifactHash(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

function validateVisualArtifacts(root) {
  for (const relative of VISUAL_ARTIFACTS) {
    const file = path.join(root, relative);
    invariant(fs.existsSync(file), `missing visual artifact: ${relative}`);
    const bytes = fs.readFileSync(file);
    invariant(bytes.length > 4, `visual artifact is empty: ${relative}`);
    if (relative.endsWith(".png")) {
      invariant(bytes[0] === 0x89 && bytes.slice(1, 4).toString("ascii") === "PNG", `invalid PNG artifact: ${relative}`);
    } else if (relative.endsWith(".ppm")) {
      invariant(bytes.slice(0, 2).toString("ascii") === "P6", `invalid PPM artifact: ${relative}`);
    }
  }
}

function validateProofRoot(root) {
  const proofRoot = path.resolve(root);
  const pf01 = validatePf01(readJson(proofRoot, JSON_ARTIFACTS.pf01));
  const fr06Browser = validateFr06Browser(readJson(proofRoot, JSON_ARTIFACTS.fr06Browser));
  const nativeSurface = validateNativeSurface(readJson(proofRoot, JSON_ARTIFACTS.nativeSurface));
  const nativeFr06 = validateNativeFr06(readJson(proofRoot, JSON_ARTIFACTS.nativeFr06));
  validateQ01Parity(readJson(proofRoot, JSON_ARTIFACTS.q01Parity), proofRoot);
  validateQ04Lifecycle(readJson(proofRoot, JSON_ARTIFACTS.q04Lifecycle));
  validateP01Benchmark(readJson(proofRoot, JSON_ARTIFACTS.p01Benchmark));
  validateVisualArtifacts(proofRoot);

  const artifactSha256 = {};
  for (const relative of [...Object.values(JSON_ARTIFACTS), ...VISUAL_ARTIFACTS]) {
    artifactSha256[relative] = artifactHash(path.join(proofRoot, relative));
  }
  const browserBackends = pf01.backends.map((backend) => backend.backend).sort();
  return {
    schema: "scena.windows_complete_hardware_proof.v1",
    generated_at: new Date().toISOString(),
    status: "passed",
    hardware_evidence: true,
    release_evidence: false,
    release_provenance: {
      status: "unavailable",
      reason: "the collected hardware artifacts are not bound to one exact source commit",
    },
    coverage: {
      browser_backends: browserBackends,
      native_surface: true,
      native_semantic_aov: true,
      required_webgpu_pixel_parity: true,
      gpu_resource_lifecycle: true,
      shader_module_cache_p95: true,
      native_surface_resize_recovery: true,
    },
    adapters: {
      browser: Object.fromEntries(pf01.backends.map((backend) => [backend.backend, {
        capability_adapter: backend.capability_report.adapter,
        browser_gpu: backend.browser_gpu,
      }])),
      native_surface: nativeSurface.adapter,
      native_semantic_aov: nativeFr06.capability_report.adapter,
    },
    pf01_phase_hashes: Object.fromEntries(pf01.backends.map((backend) => [
      backend.backend,
      Object.fromEntries(PHASE_IDS.map((id) => [id, backend.phases[id].fnv1a64])),
    ])),
    native_phase_hashes: Object.fromEntries(
      PHASE_IDS.map((id) => [id, nativeSurface.output_toggle.phases[id].fnv1a64]),
    ),
    fr06_browser_parity: fr06Browser.parity,
    native_fr06_center: nativeFr06.center,
    artifact_sha256: artifactSha256,
  };
}

function main(argv) {
  invariant(argv.length === 2, "usage: node windows_complete_hardware_proof_validation.js <proof-root> <summary-output>");
  const [proofRoot, summaryOutput] = argv;
  const summary = validateProofRoot(proofRoot);
  fs.mkdirSync(path.dirname(path.resolve(summaryOutput)), { recursive: true });
  fs.writeFileSync(summaryOutput, `${JSON.stringify(summary, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(summary)}\n`);
}

if (require.main === module) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error.stack || error);
    process.exitCode = 1;
  }
}

module.exports = {
  validateFr06Browser,
  validateNativeFr06,
  validateNativeSurface,
  validateP01Benchmark,
  validatePf01,
  validateProofRoot,
  validateQ01Parity,
  validateQ04Lifecycle,
};
