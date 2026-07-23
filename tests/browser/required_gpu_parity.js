const SOFTWARE_ADAPTER_MARKERS = [
  "swiftshader",
  "llvmpipe",
  "lavapipe",
  "software rasterizer",
  "microsoft basic render",
];

function softwareAdapter(adapter) {
  if (!adapter || typeof adapter !== "object") {
    return false;
  }
  const identity = [adapter.name, adapter.device_type, adapter.driver, adapter.driver_info]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
  return (
    String(adapter.device_type || "").toLowerCase() === "cpu" ||
    SOFTWARE_ADAPTER_MARKERS.some((marker) => identity.includes(marker))
  );
}

function hardwareAdapter(adapter) {
  const deviceType = String(adapter && adapter.device_type ? adapter.device_type : "").toLowerCase();
  return ["discretegpu", "integratedgpu", "virtualgpu"].includes(deviceType) && !softwareAdapter(adapter);
}

function browserGpuRendererIdentity(browserGpu) {
  if (!browserGpu || typeof browserGpu !== "object") return "";
  const aux = browserGpu.aux_attributes && typeof browserGpu.aux_attributes === "object"
    ? browserGpu.aux_attributes
    : {};
  return [aux.gl_vendor, aux.gl_renderer].filter(Boolean).join(" ").toLowerCase();
}

function softwareBrowserGpu(browserGpu) {
  const rendererIdentity = browserGpuRendererIdentity(browserGpu);
  if (rendererIdentity) {
    return SOFTWARE_ADAPTER_MARKERS.some((marker) => rendererIdentity.includes(marker));
  }
  const devices = Array.isArray(browserGpu && browserGpu.devices) ? browserGpu.devices : [];
  return devices.length > 0 && devices.every((device) => {
    const identity = [device && device.vendor_string, device && device.device_string]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    return SOFTWARE_ADAPTER_MARKERS.some((marker) => identity.includes(marker));
  });
}

function hardwareBrowserGpu(browserGpu) {
  if (
    !browserGpu
    || browserGpu.source !== "chromium-cdp-system-info"
    || softwareBrowserGpu(browserGpu)
  ) {
    return false;
  }
  const renderer = browserGpuRendererIdentity(browserGpu);
  if (!renderer) return false;
  const devices = Array.isArray(browserGpu.devices) ? browserGpu.devices : [];
  return devices.some((device) => {
    if (!device || typeof device !== "object") return false;
    const deviceIdentity = String(device.device_string || "").trim().toLowerCase();
    const vendorId = Number(device.vendor_id || 0);
    const deviceId = Number(device.device_id || 0);
    return deviceIdentity.length > 0
      && vendorId > 0
      && deviceId > 0
      && renderer.includes(deviceIdentity);
  });
}

function normalizedBackend(value) {
  return String(value || "").toLowerCase().replace(/[^a-z0-9]/g, "");
}

const REQUIRED_PIXEL_THRESHOLDS = Object.freeze({
  rgb_chebyshev_tolerance: 4,
  within_tolerance_fraction_min: 0.995,
  rgb_rmse_max: 2.0,
  p99_5_channel_delta_max: 4,
  foreground_iou_min: 0.995,
});

const REQUIRED_PIXEL_MUTATIONS = Object.freeze([
  "wrong-colors",
  "geometry-shift",
  "missing-object",
  "vertical-flip",
  "linear-as-srgb",
  "stale-reference",
]);

function decodeParityFrame(frame, source) {
  if (
    !frame ||
    frame.source !== source ||
    !Number.isInteger(frame.width) ||
    !Number.isInteger(frame.height) ||
    frame.width <= 0 ||
    frame.height <= 0 ||
    typeof frame.rgba8_base64 !== "string"
  ) {
    throw new Error(`missing ${source} RGBA8 frame`);
  }
  const rgba = Buffer.from(frame.rgba8_base64, "base64");
  if (rgba.length !== frame.width * frame.height * 4) {
    throw new Error(`${source} RGBA8 byte count does not match dimensions`);
  }
  return { width: frame.width, height: frame.height, rgba };
}

function pixelForeground(rgba, offset) {
  return Math.max(rgba[offset], rgba[offset + 1], rgba[offset + 2]) > 8;
}

function gradientEdgeMask(reference) {
  const { width, height } = reference;
  const edge = new Uint8Array(width * height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = y * width + x;
      const offset = index * 4;
      for (const [dx, dy] of [[1, 0], [0, 1]]) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx >= width || ny >= height) continue;
        const neighbor = (ny * width + nx) * 4;
        const referenceDelta = Math.max(
          Math.abs(reference.rgba[offset] - reference.rgba[neighbor]),
          Math.abs(reference.rgba[offset + 1] - reference.rgba[neighbor + 1]),
          Math.abs(reference.rgba[offset + 2] - reference.rgba[neighbor + 2]),
        );
        if (referenceDelta > 16) {
          edge[index] = 1;
          edge[ny * width + nx] = 1;
        }
      }
    }
  }
  const excluded = new Uint8Array(width * height);
  for (let index = 0; index < edge.length; index += 1) {
    if (!edge[index]) continue;
    const x = index % width;
    const y = Math.floor(index / width);
    for (let dy = -2; dy <= 2; dy += 1) {
      for (let dx = -2; dx <= 2; dx += 1) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx >= 0 && nx < width && ny >= 0 && ny < height) {
          excluded[ny * width + nx] = 1;
        }
      }
    }
  }
  return excluded;
}

function percentile(sorted, fraction) {
  if (sorted.length === 0) return 255;
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1)];
}

function worstRegion(reference, candidate, size = 8) {
  const width = Math.min(size, reference.width);
  const height = Math.min(size, reference.height);
  let worst = { error_sum: -1, bbox: [0, 0, width, height], rgb_rmse: 0 };
  for (let top = 0; top <= reference.height - height; top += 1) {
    for (let left = 0; left <= reference.width - width; left += 1) {
      let squared = 0;
      for (let y = top; y < top + height; y += 1) {
        for (let x = left; x < left + width; x += 1) {
          const offset = (y * reference.width + x) * 4;
          for (let channel = 0; channel < 3; channel += 1) {
            const delta = reference.rgba[offset + channel] - candidate.rgba[offset + channel];
            squared += delta * delta;
          }
        }
      }
      if (squared > worst.error_sum) {
        worst = {
          error_sum: squared,
          bbox: [left, top, width, height],
          rgb_rmse: Math.sqrt(squared / (width * height * 3)),
        };
      }
    }
  }
  return { bbox: worst.bbox, rgb_rmse: worst.rgb_rmse };
}

function evaluateFrames(reference, candidate) {
  if (
    reference.width !== candidate.width ||
    reference.height !== candidate.height ||
    reference.rgba.length !== candidate.rgba.length
  ) {
    return {
      metrics: null,
      failure_codes: ["DIMENSION_MISMATCH"],
      heatmap: Buffer.alloc(0),
      worst_region: { bbox: [0, 0, 0, 0], rgb_rmse: 255 },
      masked_edge_pixels: 0,
    };
  }
  const excluded = gradientEdgeMask(reference);
  const heatmap = Buffer.alloc(reference.rgba.length);
  const channelDeltas = [];
  let squared = 0;
  let comparedChannels = 0;
  let within = 0;
  let comparedPixels = 0;
  let foregroundIntersection = 0;
  let foregroundUnion = 0;
  let maskedEdgePixels = 0;
  for (let index = 0; index < reference.width * reference.height; index += 1) {
    const offset = index * 4;
    let chebyshev = 0;
    for (let channel = 0; channel < 3; channel += 1) {
      chebyshev = Math.max(
        chebyshev,
        Math.abs(reference.rgba[offset + channel] - candidate.rgba[offset + channel]),
      );
    }
    const heat = Math.min(255, chebyshev * 8);
    heatmap[offset] = heat;
    heatmap[offset + 1] = chebyshev <= REQUIRED_PIXEL_THRESHOLDS.rgb_chebyshev_tolerance ? 64 : 0;
    heatmap[offset + 2] = 0;
    heatmap[offset + 3] = 255;
    if (excluded[index]) {
      maskedEdgePixels += 1;
      continue;
    }
    const foregroundReference = pixelForeground(reference.rgba, offset);
    const foregroundCandidate = pixelForeground(candidate.rgba, offset);
    if (foregroundReference || foregroundCandidate) foregroundUnion += 1;
    if (foregroundReference && foregroundCandidate) foregroundIntersection += 1;
    comparedPixels += 1;
    if (chebyshev <= REQUIRED_PIXEL_THRESHOLDS.rgb_chebyshev_tolerance) within += 1;
    for (let channel = 0; channel < 3; channel += 1) {
      const delta = Math.abs(reference.rgba[offset + channel] - candidate.rgba[offset + channel]);
      channelDeltas.push(delta);
      squared += delta * delta;
      comparedChannels += 1;
    }
  }
  channelDeltas.sort((a, b) => a - b);
  const metrics = {
    compared_pixels: comparedPixels,
    masked_edge_pixels: maskedEdgePixels,
    within_tolerance_fraction: comparedPixels > 0 ? within / comparedPixels : 0,
    rgb_rmse: comparedChannels > 0 ? Math.sqrt(squared / comparedChannels) : 255,
    p99_5_channel_delta: percentile(channelDeltas, 0.995),
    foreground_iou: foregroundUnion > 0 ? foregroundIntersection / foregroundUnion : 0,
  };
  const failureCodes = [];
  if (metrics.compared_pixels === 0) failureCodes.push("EMPTY_COMPARISON_MASK");
  if (
    metrics.within_tolerance_fraction <
    REQUIRED_PIXEL_THRESHOLDS.within_tolerance_fraction_min
  ) failureCodes.push("WITHIN_TOLERANCE_FRACTION");
  if (metrics.rgb_rmse > REQUIRED_PIXEL_THRESHOLDS.rgb_rmse_max) failureCodes.push("RGB_RMSE");
  if (
    metrics.p99_5_channel_delta >
    REQUIRED_PIXEL_THRESHOLDS.p99_5_channel_delta_max
  ) failureCodes.push("P99_5_CHANNEL_DELTA");
  if (metrics.foreground_iou < REQUIRED_PIXEL_THRESHOLDS.foreground_iou_min) {
    failureCodes.push("FOREGROUND_IOU");
  }
  return {
    metrics,
    failure_codes: failureCodes,
    heatmap,
    worst_region: worstRegion(reference, candidate),
    masked_edge_pixels: maskedEdgePixels,
  };
}

function blankFrame(frame) {
  const rgba = Buffer.alloc(frame.rgba.length);
  for (let offset = 3; offset < rgba.length; offset += 4) rgba[offset] = 255;
  return { ...frame, rgba };
}

function shiftFrame(frame, dx, dy) {
  const shifted = blankFrame(frame);
  for (let y = 0; y < frame.height; y += 1) {
    for (let x = 0; x < frame.width; x += 1) {
      const nx = x + dx;
      const ny = y + dy;
      if (nx < 0 || nx >= frame.width || ny < 0 || ny >= frame.height) continue;
      frame.rgba.copy(shifted.rgba, (ny * frame.width + nx) * 4, (y * frame.width + x) * 4, (y * frame.width + x + 1) * 4);
    }
  }
  return shifted;
}

function verticalFlip(frame) {
  const flipped = blankFrame(frame);
  const rowBytes = frame.width * 4;
  for (let y = 0; y < frame.height; y += 1) {
    frame.rgba.copy(flipped.rgba, (frame.height - 1 - y) * rowBytes, y * rowBytes, (y + 1) * rowBytes);
  }
  return flipped;
}

function mutateColors(frame, linear = false) {
  const mutated = { ...frame, rgba: Buffer.from(frame.rgba) };
  for (let offset = 0; offset < mutated.rgba.length; offset += 4) {
    if (!pixelForeground(mutated.rgba, offset)) continue;
    for (let channel = 0; channel < 3; channel += 1) {
      const value = mutated.rgba[offset + channel];
      mutated.rgba[offset + channel] = linear
        ? Math.round((value / 255) ** 2.2 * 255)
        : 255 - value;
    }
  }
  return mutated;
}

function evaluateRequiredPixelParity(parity, rendererReadback) {
  if (
    !parity ||
    parity.schema !== "scena.m6.cpu_webgpu_parity.v1" ||
    normalizedBackend(parity.backend) !== "webgpu" ||
    parity.fixture?.id !== "m6-identical-unlit-triangle-v1"
  ) {
    throw new Error("missing source-bound WebGPU parity report");
  }
  const normalization = parity.normalization || {};
  if (
    normalization.row_origin !== "top-left" ||
    normalization.transfer !== "srgb8" ||
    normalization.alpha !== "straight-opaque" ||
    normalization.dimensions !== "exact" ||
    normalization.comparison_channels !== "rgb"
  ) {
    throw new Error("incomplete WebGPU parity normalization contract");
  }
  const reference = decodeParityFrame(parity.cpu_frame, "renderer-owned-cpu-frame");
  const candidate = decodeParityFrame(parity.gpu_frame, "renderer-owned-gpu-copy");
  if (
    !rendererReadback ||
    rendererReadback.source !== "renderer-owned-gpu-copy" ||
    rendererReadback.width !== candidate.width ||
    rendererReadback.height !== candidate.height ||
    rendererReadback.rgba8_base64 !== parity.gpu_frame.rgba8_base64
  ) {
    throw new Error("WebGPU parity candidate is not the renderer headline readback");
  }
  const baseline = evaluateFrames(reference, candidate);
  const mutationInputs = [
    ["wrong-colors", reference, mutateColors(candidate)],
    ["geometry-shift", reference, shiftFrame(candidate, 4, 0)],
    ["missing-object", reference, blankFrame(candidate)],
    ["vertical-flip", reference, verticalFlip(candidate)],
    ["linear-as-srgb", reference, mutateColors(candidate, true)],
    ["stale-reference", shiftFrame(reference, -8, 5), candidate],
  ];
  const mutations = mutationInputs.map(([name, mutationReference, mutationCandidate]) => {
    const evaluation = evaluateFrames(mutationReference, mutationCandidate);
    return {
      name,
      rejected: evaluation.failure_codes.length > 0,
      failure_codes: evaluation.failure_codes,
      metrics: evaluation.metrics,
    };
  });
  const failureCodes = [...baseline.failure_codes];
  if (
    mutations.length !== REQUIRED_PIXEL_MUTATIONS.length ||
    mutations.some((mutation, index) =>
      mutation.name !== REQUIRED_PIXEL_MUTATIONS[index] || mutation.rejected !== true)
  ) {
    failureCodes.push("KNOWN_BAD_MUTATION_NOT_REJECTED");
  }
  return {
    status: failureCodes.length === 0 ? "passed" : "failed",
    failure_codes: failureCodes,
    fixture: parity.fixture,
    normalization,
    thresholds: REQUIRED_PIXEL_THRESHOLDS,
    mask: {
      kind: "two-pixel-gradient-edge-exclusion",
      source: "cpu-reference-gradient",
      excluded_pixels: baseline.masked_edge_pixels,
      foreground_domain: "edge-excluded",
    },
    metrics: baseline.metrics,
    worst_region: baseline.worst_region,
    diff_heatmap_rgba8_base64: baseline.heatmap.toString("base64"),
    mutations,
  };
}

function evaluateRequiredGpuParity({ required, requestedBackend, result, error, browserGpu }) {
  const webgpuPixels = normalizedBackend(requestedBackend) === "webgpu";
  if (!required && !webgpuPixels) {
    return { status: "diagnostic", failure_codes: [] };
  }
  const message = String(error && error.message ? error.message : error || "");
  if (message.includes("NoAdapter")) {
    return required
      ? { status: "failed", failure_codes: ["NO_ADAPTER"] }
      : { status: "diagnostic", failure_codes: [], pixel_parity: null };
  }
  const failures = [];
  if (!result || typeof result !== "object") {
    return required
      ? { status: "failed", failure_codes: ["MISSING_RESULT"] }
      : { status: "diagnostic", failure_codes: [], pixel_parity: null };
  }
  if (normalizedBackend(result.backend) !== normalizedBackend(requestedBackend)) {
    failures.push("BACKEND_MISMATCH");
  }
  if (result.status !== "passed") failures.push("PROBE_NOT_PASSED");
  if (result.gpu_device !== true) failures.push("GPU_DEVICE_MISSING");
  const nonblack =
    result.renderer_readback &&
    result.renderer_readback.pixel_statistics &&
    result.renderer_readback.pixel_statistics.nonblack;
  if (
    !(result.draw_calls > 0) ||
    !(result.gpu_submissions > 0) ||
    !result.renderer_readback ||
    result.renderer_readback.source !== "renderer-owned-gpu-copy" ||
    !(nonblack > 0)
  ) {
    failures.push("ZERO_RENDERER_OUTPUT");
  }
  if (required) {
    if (softwareAdapter(result.adapter) || softwareBrowserGpu(browserGpu)) {
      failures.push("SOFTWARE_ADAPTER");
    } else if (!result.adapter || typeof result.adapter !== "object") {
      failures.push("ADAPTER_IDENTITY_MISSING");
    } else if (!hardwareAdapter(result.adapter) && !hardwareBrowserGpu(browserGpu)) {
      failures.push("ADAPTER_HARDWARE_UNPROVEN");
    }
  }
  let pixelParity = null;
  if (webgpuPixels) {
    if (!result.parity) {
      failures.push("PIXEL_PARITY_MISSING");
    } else {
      try {
        pixelParity = evaluateRequiredPixelParity(result.parity, result.renderer_readback);
        if (pixelParity.status !== "passed") failures.push("PIXEL_PARITY_MISMATCH");
      } catch (pixelError) {
        failures.push("PIXEL_PARITY_MALFORMED");
        pixelParity = {
          status: "failed",
          failure_codes: [String(pixelError && pixelError.message ? pixelError.message : pixelError)],
        };
      }
    }
  }
  return {
    status: failures.length === 0 ? (required ? "passed" : "diagnostic") : "failed",
    failure_codes: failures,
    pixel_parity: pixelParity,
  };
}

function evaluateRequiredHardwareAdapter({
  required,
  requestedBackend,
  actualBackend,
  gpuDevice,
  surfaceAttached,
  adapter,
  browserGpu,
}) {
  if (!required) {
    return { status: "diagnostic", failure_codes: [] };
  }
  const failures = [];
  if (normalizedBackend(actualBackend) !== normalizedBackend(requestedBackend)) {
    failures.push("BACKEND_MISMATCH");
  }
  if (gpuDevice !== true) failures.push("GPU_DEVICE_MISSING");
  if (surfaceAttached !== true) failures.push("SURFACE_NOT_ATTACHED");
  if (softwareAdapter(adapter) || softwareBrowserGpu(browserGpu)) {
    failures.push("SOFTWARE_ADAPTER");
  } else if (!adapter || typeof adapter !== "object") {
    failures.push("ADAPTER_IDENTITY_MISSING");
  } else if (!hardwareAdapter(adapter) && !hardwareBrowserGpu(browserGpu)) {
    failures.push("ADAPTER_HARDWARE_UNPROVEN");
  }
  return {
    status: failures.length === 0 ? "passed" : "failed",
    failure_codes: failures,
  };
}

function classifyBrowserEvidence({ required, selectedBackends, evaluations }) {
  const backends = Array.isArray(selectedBackends) ? selectedBackends.map(normalizedBackend) : [];
  const entries = Array.isArray(evaluations) ? evaluations : [];
  const webgpu = entries.find(
    (entry) => normalizedBackend(entry && entry.backend) === "webgpu",
  );
  const pixelPassed = Boolean(webgpu && webgpu.pixel_parity && webgpu.pixel_parity.status === "passed");
  const parityScope = pixelPassed ? ["webgpu:m6-identical-unlit-triangle-v1"] : [];
  const requiredPassed =
    required === true &&
    backends.includes("webgpu") &&
    pixelPassed &&
    entries.length > 0 &&
    entries.every((entry) => entry && entry.status === "passed");
  if (requiredPassed) {
    return {
      proof_class: "renderer-smoke-with-required-webgpu-full-frame-parity",
      release_evidence: true,
      parity_claim: "full-frame-reference-diff",
      parity_scope: parityScope,
    };
  }
  if (required === true) {
    return {
      proof_class: "required-webgpu-parity-failed",
      release_evidence: false,
      parity_claim: "failed",
      parity_scope: parityScope,
    };
  }
  if (pixelPassed) {
    return {
      proof_class: "renderer-conformance-with-diagnostic-webgpu-pixel-diff",
      release_evidence: false,
      parity_claim: "diagnostic-only",
      parity_scope: parityScope,
    };
  }
  return {
    proof_class: "renderer-smoke",
    release_evidence: false,
    parity_claim: "not-claimed",
    parity_scope: [],
  };
}

module.exports = {
  classifyBrowserEvidence,
  evaluateRequiredGpuParity,
  evaluateRequiredPixelParity,
  evaluateRequiredHardwareAdapter,
  hardwareBrowserGpu,
  hardwareAdapter,
  softwareBrowserGpu,
  softwareAdapter,
};
